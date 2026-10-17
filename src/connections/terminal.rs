use crate::error::{AppError, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver, SyncSender},
};

pub struct Terminal {
    pub parser: vt100::Parser<TerminalReplies>,
    pub status: String,
    pub closed: bool,
    input: SyncSender<Vec<u8>>,
    output: Receiver<Vec<u8>>,
    master: Option<Box<dyn MasterPty + Send>>,
    native: Option<std::sync::mpsc::Sender<(u16, u16)>>,
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    child: Option<Box<dyn Child + Send + Sync>>,
    pub size: (u16, u16),
}
pub struct TerminalReplies {
    input: SyncSender<Vec<u8>>,
    startup: bool,
    answered: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl vt100::Callbacks for TerminalReplies {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        _: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        if i1.is_some() {
            return;
        }
        let first = params.first().and_then(|p| p.first()).copied().unwrap_or(0);
        let response = match (c, first) {
            ('n', 6) => {
                let (r, c) = screen.cursor_position();
                format!("\x1b[{};{}R", r + 1, c + 1)
            }
            ('n', 5) => "\x1b[0n".into(),
            ('c', 0) => "\x1b[?1;2c".into(),
            _ => return,
        };
        use std::sync::atomic::Ordering;
        if self.startup {
            self.answered.fetch_add(1, Ordering::SeqCst);
        } else if self.answered.load(Ordering::SeqCst) > 0 {
            // Only the UI parser decrements; the startup reader can only increment.
            self.answered.fetch_sub(1, Ordering::SeqCst);
            return;
        }
        let _ = self.input.try_send(response.into_bytes());
    }
}
impl Terminal {
    #[cfg(test)]
    pub(crate) fn fixture() -> (Self, Receiver<Vec<u8>>) {
        let (input, rx) = mpsc::sync_channel(64);
        let (_, output) = mpsc::sync_channel(64);
        let (resize, _) = mpsc::channel();
        (
            Self {
                parser: vt100::Parser::new_with_callbacks(
                    24,
                    80,
                    100,
                    TerminalReplies {
                        input: input.clone(),
                        startup: false,
                        answered: Default::default(),
                    },
                ),
                status: String::new(),
                closed: false,
                input,
                output,
                master: None,
                native: Some(resize),
                cancel: None,
                child: None,
                size: (24, 80),
            },
            rx,
        )
    }
    pub fn from_ssh(session: ssh2::Session, ctx: eframe::egui::Context) -> Result<Self> {
        let mut channel = session
            .channel_session()
            .map_err(|e| AppError::Operation(e.to_string()))?;
        channel
            .request_pty("xterm-256color", None, Some((80, 24, 0, 0)))
            .map_err(|e| AppError::Operation(e.to_string()))?;
        channel
            .shell()
            .map_err(|e| AppError::Operation(e.to_string()))?;
        session.set_blocking(false);
        let (input, recv) = mpsc::sync_channel::<Vec<u8>>(64);
        let (send, output) = mpsc::sync_channel(64);
        let (resize, sizes) = mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopped = cancel.clone();
        std::thread::spawn(move || {
            let mut pending = zeroize::Zeroizing::new(Vec::<u8>::new());
            let mut buf = [0u8; 8192];
            let mut size = None;
            while !stopped.load(std::sync::atomic::Ordering::Acquire) {
                if pending.is_empty()
                    && let Ok(data) = recv.try_recv()
                {
                    *pending = data;
                }
                if !pending.is_empty() {
                    match channel.write(&pending) {
                        Ok(n) => {
                            use zeroize::Zeroize;
                            pending[..n].zeroize();
                            pending.drain(..n);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => break,
                    }
                }
                for next in sizes.try_iter() {
                    size = Some(next);
                }
                if let Some((rows, cols)) = size
                    && channel
                        .request_pty_size(cols as u32, rows as u32, None, None)
                        .is_ok()
                {
                    size = None;
                }
                match channel.read(&mut buf) {
                    Ok(0) => {
                        if channel.eof() {
                            break;
                        }
                    }
                    Ok(n) => {
                        if send.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => break,
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            let _ = channel.close();
            ctx.request_repaint();
        });
        Ok(Self {
            parser: vt100::Parser::new_with_callbacks(
                24,
                80,
                10000,
                TerminalReplies {
                    input: input.clone(),
                    startup: false,
                    answered: Default::default(),
                },
            ),
            status: "SSH connecté".into(),
            closed: false,
            input,
            output,
            master: None,
            native: Some(resize),
            cancel: Some(cancel),
            child: None,
            size: (24, 80),
        })
    }
    pub fn launch(
        program: &std::path::Path,
        args: &[String],
        ctx: eframe::egui::Context,
    ) -> Result<Self> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| {
                AppError::Operation("Impossible de créer le terminal système (ConPTY/PTY).".into())
            })?;
        let mut command = CommandBuilder::new(program);
        command.args(args);
        command.env("TERM", "xterm-256color");
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| AppError::Operation("Lecture du terminal indisponible.".into()))?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|_| AppError::Operation("Écriture du terminal indisponible.".into()))?;
        let (send, output) = mpsc::sync_channel(64);
        let (input, recv) = mpsc::sync_channel::<Vec<u8>>(64);
        let startup = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let answered = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reader_startup = startup.clone();
        let mut startup_parser = vt100::Parser::new_with_callbacks(
            24,
            80,
            0,
            TerminalReplies {
                input: input.clone(),
                startup: true,
                answered: answered.clone(),
            },
        );
        std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if reader_startup.load(std::sync::atomic::Ordering::SeqCst) {
                            startup_parser.process(&buffer[..n]);
                        }
                        if send.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                }
            }
        });
        std::thread::spawn(move || {
            while let Ok(mut data) = recv.recv() {
                let result = writer.write_all(&data).and_then(|_| writer.flush());
                zeroize::Zeroize::zeroize(&mut data);
                if result.is_err() {
                    break;
                }
            }
        });
        // ConPTY inherits the cursor and may wait for a DSR reply during CreateProcessW.
        // Start BOTH pipe pumps before spawning, otherwise Windows can deadlock.
        let child = pair.slave.spawn_command(command).map_err(|_| {
            AppError::Operation(
                "Impossible de lancer le programme. Vérifiez son installation.".into(),
            )
        })?;
        startup.store(false, std::sync::atomic::Ordering::SeqCst);
        drop(pair.slave);
        Ok(Self {
            parser: vt100::Parser::new_with_callbacks(
                24,
                80,
                10000,
                TerminalReplies {
                    input: input.clone(),
                    startup: false,
                    answered,
                },
            ),
            status: "Client actif — authentification dans le terminal".into(),
            closed: false,
            input,
            output,
            master: Some(pair.master),
            native: None,
            cancel: None,
            child: Some(child),
            size: (24, 80),
        })
    }
    pub fn poll(&mut self) {
        for _ in 0..128 {
            match self.output.try_recv() {
                Ok(bytes) => self.parser.process(&bytes),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if self.native.is_some() {
                        self.closed = true;
                        self.status = "SSH déconnecté".into();
                    }
                    break;
                }
                Err(_) => break,
            }
        }
        if !self.closed
            && let Some(child) = &mut self.child
        {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.closed = true;
                    self.status = format!("Déconnecté — {status}");
                }
                Ok(None) => {}
                Err(_) => {
                    self.status = "État du client indisponible".into();
                }
            }
        }
    }
    pub fn write(&mut self, bytes: Vec<u8>) {
        if !self.closed && self.input.try_send(bytes).is_err() {
            self.status = "Entrée non envoyée : terminal occupé ou fermé. Réessayez.".into();
        }
    }
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let (rows, cols) = (rows.clamp(2, 200), cols.clamp(10, 400));
        if self.size != (rows, cols)
            && let Some(native) = &self.native
        {
            let _ = native.send((rows, cols));
            self.parser.screen_mut().set_size(rows, cols);
            self.size = (rows, cols);
            return;
        }
        if self.size != (rows, cols)
            && let Some(master) = &self.master
            && master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .is_ok()
        {
            self.parser.screen_mut().set_size(rows, cols);
            self.size = (rows, cols);
        }
    }
    pub fn stop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        }
        if let Some(child) = &mut self.child {
            let _ = child.kill();
        }
        self.closed = true;
        self.status = "Session arrêtée".into();
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}
