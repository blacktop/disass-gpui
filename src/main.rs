use std::sync::Arc;
use gpui::prelude::*;
use languages::{LanguageRegistry, LanguageConfig, LanguageQueries, LoadedLanguage};
use tree_sitter_asm;

/// Embeds and registers the Zed ASM extension into the language registry.
fn register_asm_extension(registry: &LanguageRegistry) {
    // 1. Register the native Tree-sitter grammar for ASM
    registry.register_native_grammars([
        ("asm", tree_sitter_asm::language()),
    ]);

    // 2. Load the extension's config.toml (defines name, grammar, matcher, etc.)
    let config: LanguageConfig = toml::from_str(
        include_str!("../assets/zed-asm/languages/asm/config.toml")
    )
    .expect("Invalid ASM config.toml");

    // 3. Load Tree-sitter query files for highlighting (and any injections)
    let mut queries = LanguageQueries::default();
    queries.highlight = Some(include_str!(
        "../assets/zed-asm/languages/asm/highlight.scm"
    ));
    // (Add injections.scm or locals.scm if the extension provides them)

    // 4. Register the language with the registry
    registry.register_language(
        config.name.clone(),      // e.g. "Assembly"
        config.grammar.clone(),   // "asm"
        config.matcher.clone(),   // file suffix / first-line matcher
        config.hidden,
        Arc::new(move || {
            Ok(LoadedLanguage {
                config: config.clone(),
                queries: queries.clone(),
                context_provider: None,
                toolchain_provider: None,
            })
        }),
    );
}

/// Hardcoded ARM64 assembly for the demo; swap this out for dynamic input.
const ARM64_CODE: &str = r#"
    .globl _main                  // global entry
_main:
    stp     x29, x30, [sp, #-16]! // push frame pointer + LR
    mov     x29, sp
    bl      _puts                 // call puts
    mov     w0, #0x0
    ldp     x29, x30, [sp], #16   // pop frame pointer + LR
    ret
"#;

/// Maps common highlight capture names to RGB theme colors.
struct ThemeColors {
    background: u32,
    default_text: u32,
    keyword: u32,
    comment: u32,
    register: u32,
    number: u32,
    label: u32,
}
impl ThemeColors {
    fn dark_theme() -> Self {
        Self {
            background: 0x1e1e1e,
            default_text: 0xd4d4d4,
            keyword: 0x569cd6,
            comment: 0x6a9955,
            register: 0x4ec9b0,
            number: 0xb5cea8,
            label: 0xdcdcaa,
        }
    }
}

/// The main application state, holds highlighted lines.
struct AssemblyViewer {
    lines: Vec<Vec<zed_syntax::HighlightSegment>>,
    theme: ThemeColors,
}

impl AssemblyViewer {
    fn new(source: &str, theme: ThemeColors, registry: &LanguageRegistry) -> Self {
        // Lookup the ASM language and highlight the source text
        let asm_lang = registry
            .language("asm")
            .expect("ASM language not found in registry");
        let lines = asm_lang.highlight(source);

        AssemblyViewer { lines, theme }
    }
}

impl Render for AssemblyViewer {
    fn render(&mut self, _win: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(self.theme.background))
            .text_color(rgb(self.theme.default_text))
            .child(
                gpui::uniform_list(
                    cx.entity().clone(),
                    "asm_lines",
                    self.lines.len(),
                    move |viewer, range, _win, _cx| {
                        let mut items = Vec::new();
                        for i in range {
                            let mut line = div().flex();
                            for seg in &viewer.lines[i] {
                                let color = match seg.capture.as_str() {
                                    "label"    => viewer.theme.label,
                                    "keyword"  => viewer.theme.keyword,
                                    "register" => viewer.theme.register,
                                    "number"   => viewer.theme.number,
                                    "comment"  => viewer.theme.comment,
                                    _           => viewer.theme.default_text,
                                };
                                line = line.child(
                                    div().text_color(rgb(color)).child(seg.text.as_str()),
                                );
                            }
                            items.push(line);
                        }
                        items
                    },
                )
                .h_full()
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        // Create and configure the language registry
        let languages = Arc::new(LanguageRegistry::new(cx.background_executor()));
        // Ensure the registry has our ASM extension
        register_asm_extension(&languages);
        // Initialize the language registry with the application context
        languages.init(cx);
        // Open the main window
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            move |_, _window_cx| {
                cx.new(|_model_cx| AssemblyViewer::new(ARM64_CODE, ThemeColors::dark_theme(), &languages))
            },
        )
        .unwrap();
    });
}
