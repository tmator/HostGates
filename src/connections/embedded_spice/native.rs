//! Minimal stable C ABI bindings. No native pointer leaves this worker thread.
//! Libraries remain loaded for process lifetime: GLib registers static GTypes.
use super::*;
use libloading::Library;
use std::{
    ffi::{c_char, c_int, c_uint, c_ulong, c_void},
    ptr::{null, null_mut},
    sync::{OnceLock, mpsc},
    time::Duration,
};
type Obj = *mut c_void;
#[repr(C)]
struct List {
    data: Obj,
    next: *mut List,
    prev: *mut List,
}
#[repr(C)]
#[derive(Default)]
struct Primary {
    format: c_int,
    width: c_int,
    height: c_int,
    stride: c_int,
    shmid: c_int,
    data: *mut u8,
    marked: c_int,
}
macro_rules! api {
    ($($field:ident : $ty:ty = $lib:ident, $symbol:literal;)*) => {
        struct Api { $($field:$ty,)* _libs:Vec<Library> }
        impl Api { unsafe fn load(path:&Path)->Result<Self> {
            let dir=path.parent().ok_or_else(||error("SPICE : dossier natif invalide."))?;
            #[cfg(windows)] let (glib_name,object_name)=("libglib-2.0-0.dll","libgobject-2.0-0.dll");
            #[cfg(not(windows))] let (glib_name,object_name)=("libglib-2.0.so.0","libgobject-2.0.so.0");
            // Absolute paths; Windows dependencies are restricted to this DLL directory and system locations.
            let spice=unsafe { library(path)? };
            let glib=unsafe { library(&dir.join(glib_name))? };
            let object=unsafe { library(&dir.join(object_name))? };
            Ok(Self { $($field:unsafe { *(match stringify!($lib) { "spice"=>&spice,"glib"=>&glib,_=>&object }).get::<$ty>(concat!($symbol,"\0").as_bytes()).map_err(|_|error(concat!("SPICE : bibliothèque incompatible, symbole absent : ",$symbol)))? },)* _libs:vec![spice,object,glib] })
        }}
    }
}
unsafe fn library(path: &Path) -> Result<Library> {
    #[cfg(windows)]
    let result = unsafe {
        libloading::os::windows::Library::load_with_flags(path, 0x100 | 0x1000).map(Library::from)
    };
    #[cfg(not(windows))]
    let result = unsafe { Library::new(path) };
    result.map_err(|_|error("SPICE : chargement de la bibliothèque impossible. Vérifiez ses dépendances et l’architecture 64 bits."))
}
api! {
    new:unsafe extern "C" fn()->Obj = spice,"spice_session_new";
    connect:unsafe extern "C" fn(Obj)->c_int = spice,"spice_session_connect";
    disconnect:unsafe extern "C" fn(Obj) = spice,"spice_session_disconnect";
    channels:unsafe extern "C" fn(Obj)->*mut List = spice,"spice_session_get_channels";
    channel_connect:unsafe extern "C" fn(Obj)->c_int = spice,"spice_channel_connect";
    primary:unsafe extern "C" fn(Obj,c_uint,*mut Primary)->c_int = spice,"spice_display_channel_get_primary";
    mouse_mode:unsafe extern "C" fn(Obj,c_int) = spice,"spice_main_channel_request_mouse_mode";
    motion:unsafe extern "C" fn(Obj,c_int,c_int,c_int) = spice,"spice_inputs_channel_motion";
    position:unsafe extern "C" fn(Obj,c_int,c_int,c_int,c_int) = spice,"spice_inputs_channel_position";
    button_down:unsafe extern "C" fn(Obj,c_int,c_int) = spice,"spice_inputs_channel_button_press";
    button_up:unsafe extern "C" fn(Obj,c_int,c_int) = spice,"spice_inputs_channel_button_release";
    key_down:unsafe extern "C" fn(Obj,c_uint) = spice,"spice_inputs_channel_key_press";
    key_up:unsafe extern "C" fn(Obj,c_uint) = spice,"spice_inputs_channel_key_release";
    set:unsafe extern "C" fn(Obj,*const c_char,...) = object,"g_object_set";
    get:unsafe extern "C" fn(Obj,*const c_char,...) = object,"g_object_get";
    reference:unsafe extern "C" fn(Obj)->Obj = object,"g_object_ref";
    unref:unsafe extern "C" fn(Obj) = object,"g_object_unref";
    signal:unsafe extern "C" fn(Obj,*const c_char,*const c_void,Obj,Obj,c_int)->c_ulong = object,"g_signal_connect_data";
    unsignal:unsafe extern "C" fn(Obj,c_ulong) = object,"g_signal_handler_disconnect";
    iteration:unsafe extern "C" fn(Obj,c_int)->c_int = glib,"g_main_context_iteration";
    list_free:unsafe extern "C" fn(*mut List) = glib,"g_list_free";
    bytes_new:unsafe extern "C" fn()->Obj = glib,"g_byte_array_new";
    bytes_append:unsafe extern "C" fn(Obj,*const u8,c_uint)->Obj = glib,"g_byte_array_append";
    bytes_unref:unsafe extern "C" fn(Obj) = glib,"g_byte_array_unref";
}
struct Request {
    config: SpiceConfig,
    path: PathBuf,
    shared: Arc<Shared>,
}
static WORKER: OnceLock<mpsc::SyncSender<Request>> = OnceLock::new();
pub(super) fn start(config: SpiceConfig, path: PathBuf, shared: Arc<Shared>) -> Result<()> {
    let tx = WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel(32);
        std::thread::spawn(move || worker(rx));
        tx
    });
    tx.try_send(Request {
        config,
        path,
        shared,
    })
    .map_err(|_| error("SPICE : moteur occupé ou indisponible."))
}
struct ChannelData {
    shared: Arc<Shared>,
    kind: i32,
    id: i32,
    opened: AtomicBool,
}
struct Channel {
    object: Obj,
    data: Box<ChannelData>,
    signals: Vec<c_ulong>,
}
struct Session {
    object: Obj,
    channels: Vec<Channel>,
    shared: Arc<Shared>,
    since: Instant,
    last_frame: Instant,
    mouse_requested: bool,
    last_position: Option<(i32, i32)>,
    received_frame: bool,
}
unsafe extern "C" fn event(_channel: Obj, event: c_int, data: Obj) {
    // Signal userdata is owned by Channel and disconnected before that Box is dropped.
    let data = unsafe { &*data.cast::<ChannelData>() };
    if event == 10 {
        data.opened.store(true, Ordering::Release);
    }
    if event >= 20 || (event == 12 && data.kind == 1) {
        let message = match event {
            20 => "SPICE : connexion au proxy/serveur impossible.",
            21 => "SPICE : certificat TLS refusé (CA ou sujet du serveur).",
            22 => "SPICE : négociation refusée.",
            23 => "SPICE : ticket refusé ou expiré. Relancez la connexion.",
            24 => "SPICE : connexion interrompue.",
            _ => "SPICE : connexion fermée.",
        };
        data.shared.status(message, true);
        data.shared.stop.store(true, Ordering::Release);
    }
}
unsafe extern "C" fn invalidate(
    _channel: Obj,
    _x: c_int,
    _y: c_int,
    _w: c_int,
    _h: c_int,
    data: Obj,
) {
    unsafe { &*data.cast::<ChannelData>() }
        .shared
        .dirty
        .store(true, Ordering::Release);
}
unsafe extern "C" fn primary_create(
    _channel: Obj,
    _format: c_int,
    _w: c_int,
    _h: c_int,
    _stride: c_int,
    _shmid: c_int,
    _pixels: Obj,
    data: Obj,
) {
    unsafe { &*data.cast::<ChannelData>() }
        .shared
        .dirty
        .store(true, Ordering::Release);
}
unsafe extern "C" fn cursor_shape(
    _channel: Obj,
    w: c_int,
    h: c_int,
    x: c_int,
    y: c_int,
    pixels: Obj,
    data: Obj,
) {
    let shared = &unsafe { &*data.cast::<ChannelData>() }.shared;
    if w <= 0
        || h <= 0
        || w > 512
        || h > 512
        || x < 0
        || y < 0
        || x >= w
        || y >= h
        || pixels.is_null()
    {
        return;
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(pixels.cast::<u8>(), w as usize * h as usize * 4) };
    let colors = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[2], p[1], p[0], p[3]))
        .collect();
    shared
        .state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .cursor = Some(Cursor::Shape(
        egui::ColorImage::new([w as usize, h as usize], colors),
        [x as f32, y as f32],
    ));
    shared.ctx.request_repaint();
}
unsafe extern "C" fn cursor_hide(_channel: Obj, data: Obj) {
    let shared = &unsafe { &*data.cast::<ChannelData>() }.shared;
    shared
        .state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .cursor = Some(Cursor::Hidden);
    shared.ctx.request_repaint();
}
unsafe extern "C" fn cursor_reset(_channel: Obj, data: Obj) {
    let shared = &unsafe { &*data.cast::<ChannelData>() }.shared;
    shared
        .state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .cursor = Some(Cursor::Default);
    shared.ctx.request_repaint();
}
impl Session {
    unsafe fn new(api: &Api, request: Request) -> Result<Self> {
        let config = request.config;
        let object = unsafe { (api.new)() };
        if object.is_null() {
            return Err(error("SPICE : création de session impossible."));
        }
        // Validated scalars contain no NUL. GObject copies strings and boxed byte arrays.
        for (name, value) in [
            (c"host", &*config.host),
            (c"proxy", &config.proxy),
            (c"password", &*config.password),
            (c"cert-subject", &config.subject),
            (c"tls-port", &config.tls_port.to_string()),
        ] {
            let mut bytes = value.as_bytes().to_vec();
            bytes.push(0);
            unsafe {
                (api.set)(
                    object,
                    name.as_ptr(),
                    bytes.as_ptr().cast::<c_char>(),
                    null::<c_char>(),
                );
            }
            use zeroize::Zeroize;
            bytes.zeroize();
        }
        let ca = config.ca.replace("\\n", "\n");
        unsafe {
            let array = (api.bytes_new)();
            (api.bytes_append)(array, ca.as_ptr(), ca.len() as c_uint);
            (api.set)(object, c"ca".as_ptr(), array, null::<c_char>());
            (api.bytes_unref)(array);
            let channels = [c"all".as_ptr(), null()];
            // SUBJECT validates the certificate chain using ca, then the Proxmox-provided subject.
            // HOSTNAME would incorrectly compare against Proxmox's opaque proxy-ticket host.
            (api.set)(
                object,
                c"verify".as_ptr(),
                4u32,
                c"secure-channels".as_ptr(),
                channels.as_ptr(),
                c"enable-audio".as_ptr(),
                0i32,
                c"enable-usbredir".as_ptr(),
                0i32,
                c"enable-smartcard".as_ptr(),
                0i32,
                null::<c_char>(),
            );
        }
        #[cfg(unix)]
        unsafe {
            (api.set)(object, c"gl-scanout".as_ptr(), 0i32, null::<c_char>());
        }
        let mut session = Self {
            object,
            channels: Vec::new(),
            shared: request.shared,
            since: Instant::now(),
            last_frame: Instant::now(),
            mouse_requested: false,
            last_position: None,
            received_frame: false,
        };
        if unsafe { (api.connect)(object) } == 0 {
            unsafe { (api.unref)(object) };
            return Err(error("SPICE : configuration de connexion refusée."));
        }
        unsafe {
            session.discover(api);
        }
        Ok(session)
    }
    unsafe fn discover(&mut self, api: &Api) {
        let list = unsafe { (api.channels)(self.object) };
        let mut node = list;
        while !node.is_null() {
            let object = unsafe { (*node).data };
            if !self.channels.iter().any(|c| c.object == object) {
                if self.channels.len() >= 64 {
                    self.shared.status("SPICE : trop de canaux distants.", true);
                    self.shared.stop.store(true, Ordering::Release);
                    break;
                }
                let mut kind = 0i32;
                let mut id = 0i32;
                unsafe {
                    (api.get)(
                        object,
                        c"channel-type".as_ptr(),
                        &mut kind,
                        c"channel-id".as_ptr(),
                        &mut id,
                        null::<c_char>(),
                    );
                }
                let mut data = Box::new(ChannelData {
                    shared: self.shared.clone(),
                    kind,
                    id,
                    opened: AtomicBool::new(false),
                });
                let user = (&mut *data as *mut ChannelData).cast();
                let mut signals = vec![unsafe {
                    (api.signal)(
                        object,
                        c"channel-event".as_ptr(),
                        event as *const c_void,
                        user,
                        null_mut(),
                        0,
                    )
                }];
                if kind == 2 && id == 0 {
                    signals.push(unsafe {
                        (api.signal)(
                            object,
                            c"display-invalidate".as_ptr(),
                            invalidate as *const c_void,
                            user,
                            null_mut(),
                            0,
                        )
                    });
                    signals.push(unsafe {
                        (api.signal)(
                            object,
                            c"display-primary-create".as_ptr(),
                            primary_create as *const c_void,
                            user,
                            null_mut(),
                            0,
                        )
                    });
                }
                if kind == 4 && id == 0 {
                    for (name, callback) in [
                        (c"cursor-set", cursor_shape as *const c_void),
                        (c"cursor-hide", cursor_hide as *const c_void),
                        (c"cursor-reset", cursor_reset as *const c_void),
                    ] {
                        signals.push(unsafe {
                            (api.signal)(object, name.as_ptr(), callback, user, null_mut(), 0)
                        });
                    }
                }
                unsafe {
                    (api.reference)(object);
                }
                self.channels.push(Channel {
                    object,
                    data,
                    signals,
                });
                if (matches!(kind, 2 | 4) && id == 0) || kind == 3 {
                    unsafe {
                        (api.channel_connect)(object);
                    }
                }
            }
            node = unsafe { (*node).next };
        }
        unsafe {
            (api.list_free)(list);
        }
    }
    unsafe fn tick(&mut self, api: &Api) {
        unsafe {
            self.discover(api);
        }
        if !self.mouse_requested
            && let Some(main) = self
                .channels
                .iter()
                .find(|c| c.data.kind == 1 && c.data.opened.load(Ordering::Acquire))
        {
            unsafe {
                (api.mouse_mode)(main.object, 2);
            }
            self.mouse_requested = true;
        }
        if let Some(input) = self
            .channels
            .iter()
            .find(|c| c.data.kind == 3 && c.data.opened.load(Ordering::Acquire))
        {
            let mut mouse_mode = 2i32;
            if let Some(main) = self.channels.iter().find(|c| c.data.kind == 1) {
                unsafe {
                    (api.get)(
                        main.object,
                        c"mouse-mode".as_ptr(),
                        &mut mouse_mode,
                        null::<c_char>(),
                    );
                }
            }
            let events =
                std::mem::take(&mut *self.shared.input.lock().unwrap_or_else(|e| e.into_inner()));
            for event in events {
                unsafe {
                    match event {
                        Input::Position(x, y, buttons) => {
                            if mouse_mode == 2 {
                                (api.position)(input.object, x, y, 0, buttons);
                            } else if let Some((old_x, old_y)) = self.last_position {
                                (api.motion)(input.object, x - old_x, y - old_y, buttons);
                            }
                            self.last_position = Some((x, y));
                        }
                        Input::Button(b, true, s) => (api.button_down)(input.object, b, s),
                        Input::Button(b, false, s) => (api.button_up)(input.object, b, s),
                        Input::Key(k, true) => (api.key_down)(input.object, k),
                        Input::Key(k, false) => (api.key_up)(input.object, k),
                    }
                }
            }
        }
        if self.last_frame.elapsed() < Duration::from_millis(33) {
            return;
        }
        self.last_frame = Instant::now();
        if self.shared.dirty.swap(false, Ordering::AcqRel)
            && let Some(display) = self
                .channels
                .iter()
                .find(|c| c.data.kind == 2 && c.data.id == 0)
        {
            let mut primary = Primary::default();
            if unsafe { (api.primary)(display.object, 0, &mut primary) } != 0 {
                match unsafe { copy_frame(&primary) } {
                    Ok(frame) => {
                        self.received_frame = true;
                        let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
                        state.frame = Some(frame);
                        state.status = "SPICE : connecté".into();
                        drop(state);
                        self.shared.ctx.request_repaint();
                    }
                    Err(e) => {
                        self.shared.status(&e.to_string(), true);
                        self.shared.stop.store(true, Ordering::Release);
                    }
                }
            }
        }
        if !self.received_frame && self.since.elapsed() > Duration::from_secs(30) {
            self.shared
                .status("SPICE : délai de connexion dépassé.", true);
            self.shared.stop.store(true, Ordering::Release);
        }
    }
    unsafe fn close(self, api: &Api) {
        for channel in &self.channels {
            for &signal in &channel.signals {
                unsafe {
                    (api.unsignal)(channel.object, signal);
                }
            }
        }
        unsafe {
            (api.disconnect)(self.object);
        }
        for channel in &self.channels {
            unsafe {
                (api.unref)(channel.object);
            }
        }
        unsafe {
            (api.unref)(self.object);
        }
        let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.closed {
            state.status = "SPICE : connexion fermée.".into();
            state.closed = true;
        }
        drop(state);
        self.shared.ctx.request_repaint();
    }
}
unsafe fn copy_frame(p: &Primary) -> Result<egui::ColorImage> {
    if p.width <= 0
        || p.height <= 0
        || p.width > 4096
        || p.height > 4096
        || p.data.is_null()
        || !matches!(p.format, 32 | 96 | 16 | 80)
    {
        return Err(error(
            "SPICE : format d’affichage non pris en charge (maximum 4096 × 4096).",
        ));
    }
    let bpp = if matches!(p.format, 32 | 96) { 4 } else { 2 };
    if p.stride.unsigned_abs() < p.width as u32 * bpp || p.stride.unsigned_abs() > 65536 {
        return Err(error("SPICE : surface invalide."));
    }
    let mut pixels = Vec::with_capacity(p.width as usize * p.height as usize);
    for y in 0..p.height {
        // get_primary guarantees valid rows until the next GLib iteration. Negative strides are legal.
        let row = unsafe {
            std::slice::from_raw_parts(
                p.data.offset(y as isize * p.stride as isize),
                p.width as usize * bpp as usize,
            )
        };
        for pixel in row.chunks_exact(bpp as usize) {
            let (r, g, b) = if bpp == 4 {
                (pixel[2], pixel[1], pixel[0])
            } else {
                let v = u16::from_le_bytes([pixel[0], pixel[1]]);
                if p.format == 80 {
                    (
                        ((v >> 11) & 31) as u8 * 8,
                        ((v >> 5) & 63) as u8 * 4,
                        (v & 31) as u8 * 8,
                    )
                } else {
                    (
                        ((v >> 10) & 31) as u8 * 8,
                        ((v >> 5) & 31) as u8 * 8,
                        (v & 31) as u8 * 8,
                    )
                }
            };
            pixels.push(egui::Color32::from_rgb(r, g, b));
        }
    }
    Ok(egui::ColorImage::new(
        [p.width as usize, p.height as usize],
        pixels,
    ))
}
fn worker(rx: mpsc::Receiver<Request>) {
    let mut api: Option<&'static Api> = None;
    let mut runtime: Option<PathBuf> = None;
    let mut sessions: Vec<Session> = Vec::new();
    loop {
        // Keep dispatching asynchronous GIO cancellation after the last session closes.
        // A sleeping 100 ms idle interval avoids retaining native credential copies indefinitely.
        let delay = if sessions.is_empty() { 100 } else { 8 };
        let request = rx.recv_timeout(Duration::from_millis(delay)).ok();
        if let Some(request) = request {
            if request.shared.stop.load(Ordering::Acquire) {
                continue;
            }
            if api.is_none() {
                match unsafe { Api::load(&request.path) } {
                    Ok(loaded) => {
                        api = Some(Box::leak(Box::new(loaded)));
                        runtime = Some(request.path.clone());
                    }
                    Err(e) => {
                        request.shared.status(&e.to_string(), true);
                        continue;
                    }
                }
            }
            if runtime.as_ref() != Some(&request.path) {
                request.shared.status(
                    "SPICE : redémarrez HostGates après un changement de bibliothèque native.",
                    true,
                );
                continue;
            }
            let shared = request.shared.clone();
            if let Some(api) = api {
                match unsafe { Session::new(api, request) } {
                    Ok(s) => sessions.push(s),
                    Err(e) => shared.status(&e.to_string(), true),
                }
            }
        }
        if let Some(api) = api {
            for _ in 0..128 {
                if unsafe { (api.iteration)(null_mut(), 0) } == 0 {
                    break;
                }
            }
            for session in &mut sessions {
                unsafe {
                    session.tick(api);
                }
            }
            let mut index = 0;
            while index < sessions.len() {
                if sessions[index].shared.stop.load(Ordering::Acquire) {
                    unsafe {
                        sessions.remove(index).close(api);
                    }
                } else {
                    index += 1;
                }
            }
            // Disconnect is scheduled as a GLib idle. Drain it before sleeping with no sessions.
            for _ in 0..128 {
                if unsafe { (api.iteration)(null_mut(), 0) } == 0 {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_surface_conversion_handles_padding_and_negative_stride() {
        let mut pixels = vec![3, 2, 1, 0, 0, 0, 0, 0, 6, 5, 4, 0, 0, 0, 0, 0];
        let primary = Primary {
            format: 32,
            width: 1,
            height: 2,
            stride: 8,
            data: pixels.as_mut_ptr(),
            ..Default::default()
        };
        let frame = unsafe { copy_frame(&primary) }.expect("surface");
        assert_eq!(
            frame.pixels,
            vec![
                egui::Color32::from_rgb(1, 2, 3),
                egui::Color32::from_rgb(4, 5, 6)
            ]
        );
        let flipped = Primary {
            stride: -8,
            data: unsafe { pixels.as_mut_ptr().add(8) },
            ..primary
        };
        assert_eq!(
            unsafe { copy_frame(&flipped) }
                .expect("negative stride")
                .pixels[0],
            frame.pixels[1]
        );
        assert!(
            unsafe {
                copy_frame(&Primary {
                    width: 4097,
                    ..primary
                })
            }
            .is_err()
        );
        assert!(
            unsafe {
                copy_frame(&Primary {
                    stride: 1,
                    ..primary
                })
            }
            .is_err()
        );
    }
    #[test]
    #[ignore = "requires local QEMU TLS fixture; HOSTGATES_SPICE_FIXTURE is synthetic configuration JSON"]
    fn native_qemu_tls_proxy_frame_and_disconnect() {
        let fixture = std::env::var("HOSTGATES_SPICE_FIXTURE").expect("fixture path");
        let fixture_bytes = std::fs::read(fixture).expect("fixture file");
        let config: SpiceConfig = serde_json::from_slice(&fixture_bytes).expect("fixture JSON");
        let ctx = egui::Context::default();
        let runtime = runtime_path("").expect("installed native library");
        let mut session = EmbeddedSpice::new(config, &runtime, &ctx).expect("start session");
        let since = Instant::now();
        loop {
            session.poll(&ctx);
            assert!(!session.closed, "{}", session.status);
            if session.texture.is_some() {
                break;
            }
            assert!(
                since.elapsed() < Duration::from_secs(20),
                "frame timeout: {}",
                session.status
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(session.size[0] >= 320 && session.size[1] >= 200);
        session.key(0x2a, true);
        session.release_input();
        assert!(session.keys.is_empty());
        session.shared.send(Input::Key(0x1c, true));
        session.shared.send(Input::Key(0x1c, false));
        session.shared.send(Input::Position(100, 100, 0));
        session.shared.send(Input::Button(1, true, 1));
        session.shared.send(Input::Button(1, false, 0));
        std::thread::sleep(Duration::from_millis(100));
        session.stop();
        let since = Instant::now();
        loop {
            session.poll(&ctx);
            if session.status.contains("fermée") {
                break;
            }
            assert!(
                since.elapsed() < Duration::from_secs(3),
                "disconnect timeout"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        for wrong_ca in [false, true] {
            let mut config: SpiceConfig =
                serde_json::from_slice(&fixture_bytes).expect("fixture JSON");
            if wrong_ca {
                config.ca = rcgen::generate_simple_self_signed(vec!["untrusted.test".into()])
                    .expect("test CA")
                    .cert
                    .pem();
            } else {
                config.subject = "CN=wrong-server".into();
            }
            let mut rejected =
                EmbeddedSpice::new(config, &runtime, &ctx).expect("negative TLS test");
            let since = Instant::now();
            loop {
                rejected.poll(&ctx);
                assert!(rejected.texture.is_none(), "TLS verification bypassed");
                if rejected.closed {
                    break;
                }
                assert!(
                    since.elapsed() < Duration::from_secs(10),
                    "TLS rejection timeout"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(rejected.status.contains("TLS"), "{}", rejected.status);
        }
    }
}
