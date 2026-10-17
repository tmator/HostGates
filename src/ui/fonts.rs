use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

// Bundled OFL fonts: no network or dependency on Windows system font files.
const CJK: &[u8] = include_bytes!("../../assets/fonts/NotoSansCJK-Regular.ttc");
const ARABIC: &[u8] = include_bytes!("../../assets/fonts/NotoSansArabic.ttf");
const DEVANAGARI: &[u8] = include_bytes!("../../assets/fonts/NotoSansDevanagari.ttf");

fn definitions(language: &str) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    // Noto CJK collection: Japanese=0, Korean=1, Simplified Chinese=2.
    let cjk_index = match language {
        "ko" => 1,
        "zh-CN" => 2,
        _ => 0,
    };
    for (name, bytes, index) in [
        ("noto-cjk", CJK, cjk_index),
        ("noto-arabic", ARABIC, 0),
        ("noto-devanagari", DEVANAGARI, 0),
    ] {
        let mut data = FontData::from_static(bytes);
        data.index = index;
        fonts.font_data.insert(name.into(), data.into());
    }
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let fallback = fonts.families.entry(family.clone()).or_default();
        fallback.extend(["noto-arabic", "noto-devanagari", "noto-cjk"].map(str::to_owned));
        // Keep spaces and combining marks in the same shaping run as the text.
        // Terminal cells keep their monospace primary font.
        if family == FontFamily::Proportional {
            let primary = match language {
                "ar" => Some("noto-arabic"),
                "hi" => Some("noto-devanagari"),
                _ => None,
            };
            if let Some(primary) = primary {
                fallback.retain(|name| name != primary);
                fallback.insert(0, primary.into());
            }
        }
    }
    fonts
}

pub(super) fn install(ctx: &egui::Context, language: &str) {
    ctx.set_fonts(definitions(language));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_layout_all_new_scripts() {
        for (code, sample) in [
            ("ja", "接続 パスワード"),
            ("zh-CN", "连接 密码"),
            ("ko", "연결 비밀번호"),
            ("ar", "كلمة المرور"),
            ("hi", "पासवर्ड सहेजें"),
        ] {
            let ctx = egui::Context::default();
            install(&ctx, code);
            let mut output = ctx.run_ui(Default::default(), |ui| {
                ui.ctx().fonts_mut(|fonts| {
                    assert!(
                        fonts.has_glyphs(&egui::FontId::proportional(16.0), sample),
                        "missing glyphs: {code}"
                    );
                    let galley = fonts.layout_no_wrap(
                        sample.into(),
                        egui::FontId::proportional(16.0),
                        egui::Color32::WHITE,
                    );
                    assert!(galley.size().x > 0.0, "{code}");
                    assert!(!galley.rows.is_empty(), "{code}");
                });
            });
            output.textures_delta.clear();
        }
    }
}
