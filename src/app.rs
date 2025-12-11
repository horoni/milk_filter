use crate::filt::MilkImage;
use egui::{Color32, RichText};
use std::future::Future;
use std::sync::mpsc::{Receiver, Sender, channel};

struct FileDN {
    name: String,
    data: Vec<u8>,
    exists: bool,
    valid: bool,
}

impl FileDN {
    fn new(name: String, data: Vec<u8>, valid: bool) -> Self {
        Self {
            name,
            data,
            exists: true,
            valid,
        }
    }
}

impl Default for FileDN {
    fn default() -> Self {
        Self {
            name: String::new(),
            data: Vec::new(),
            exists: false,
            valid: false,
        }
    }
}

pub struct MilkApp {
    show_about: bool,
    show_config: bool,
    file_ch: (Sender<FileDN>, Receiver<FileDN>),
    file: FileDN,
    img: MilkImage,
    texture: Option<egui::TextureHandle>,
}

impl Default for MilkApp {
    fn default() -> Self {
        Self {
            show_about: false,
            show_config: false,
            file_ch: channel(),
            file: FileDN::default(),
            img: MilkImage::new(),
            texture: None,
        }
    }
}

impl MilkApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        Default::default()
    }
}

impl eframe::App for MilkApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        puffin::profile_function!();
        puffin::GlobalProfiler::lock().new_frame();

        if let Ok(file) = self.file_ch.1.try_recv() {
            self.file = file;
            if self.file.valid {
                puffin::profile_scope!("s_load_and_process_image");
                self.img.open(&self.file.data);
                self.img.process();

                if let Some(img) = &self.img.processed {
                    puffin::profile_scope!("s_load_texture");
                    let color_image = egui::ColorImage::from_rgb(
                        [img.width() as usize, img.height() as usize],
                        img.as_raw().as_slice(),
                    );

                    self.texture = Some(ctx.load_texture(
                        "image",
                        color_image,
                        egui::TextureOptions::default(),
                    ));
                } else {
                    self.texture = None;
                }
            }
        }

        self.window_about(ctx);
        self.window_config(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                let str = format!(
                    "milk_filter v{}",
                    option_env!("CARGO_PKG_VERSION").unwrap_or("(Undefined)")
                );
                ui.heading(RichText::new(str).color(Color32::WHITE));
            });

            load_save_file(self, ui);

            ui.horizontal(|ui| {
                let mut profile = puffin::are_scopes_on();
                ui.checkbox(&mut profile, "Show profiler window");
                puffin::set_scopes_on(profile); // controls both the profile capturing, and the displaying of it

                ui.checkbox(&mut self.show_config, "Show Filter Options");
            });

            ui.separator();

            if !self.file.exists {
                ui.label("Load file by using \"Load\" button!");
            } else if !self.file.valid {
                ui.label(RichText::new("(png | jpg | jpeg) Only supported!").color(Color32::RED));
            } else {
                ui.horizontal(|ui| {
                    ui.label("editing: ");
                    ui.label(RichText::new(&self.file.name).color(Color32::GREEN));
                });
            }

            if let Some(tex) = &self.texture {
                puffin::profile_scope!("s_draw_img");
                let max_size = ui.available_size() * 0.9;
                ui.add(egui::Image::new(tex).max_size(max_size));
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                if ui.button("About").clicked() {
                    self.show_about = true;
                }
                egui::warn_if_debug_build(ui);
            });
        });
        if puffin::are_scopes_on() {
            puffin_egui::profiler_window(ctx);
        }
    }
}

struct LogEntry {
    date: &'static str,
    text: &'static str,
}

impl MilkApp {
    fn window_about(&mut self, ctx: &egui::Context) {
        use std::sync::OnceLock;
        const CHANGELOG: &str = include_str!("../CHANGELOG.txt");
        static PARSED_LOG: OnceLock<Vec<LogEntry>> = std::sync::OnceLock::new();

        let entries = PARSED_LOG.get_or_init(|| {
            CHANGELOG
                .lines()
                .filter_map(|line| line.split_once("->"))
                .map(|(d, t)| LogEntry { date: d, text: t })
                .collect()
        });

        let rect = ctx.content_rect();
        let max_width = if rect.width() < 500.0 {
            rect.width() * 0.9
        } else {
            500.0
        };

        egui::Window::new("About")
            .open(&mut self.show_about)
            .vscroll(true)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .collapsible(false)
            .default_width(max_width)
            .max_width(max_width)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.heading(RichText::new("Changelog").color(Color32::WHITE));
                });
                for entry in entries {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("{}->", entry.date))
                                .color(Color32::YELLOW)
                                .monospace(),
                        );
                        ui.label(RichText::new(entry.text).color(Color32::GREEN));
                    });
                }
            });
    }

    fn window_config(&mut self, ctx: &egui::Context) {
        egui::Window::new("Config")
            .open(&mut self.show_config)
            .vscroll(true)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .collapsible(true)
            .default_size(egui::vec2(200.0, 200.0))
            .show(ctx, |ui| {
                let conf = self.img.get_config();

                if ui.checkbox(&mut conf.alt, "Alternative pallete").changed()
                    || ui.checkbox(&mut conf.pointism, "Pointillism").changed()
                    || ui.checkbox(&mut conf.enabled, "Milk Enabled").changed()
                    || ui.checkbox(&mut conf.quant, "Quant").changed()
                    || ui.checkbox(&mut conf.block, "Blocks").changed()
                    || ui
                        .add(
                            egui::Slider::new(&mut conf.comp, 0..=100)
                                .text("(Slow) Compression")
                                .suffix(" %"),
                        )
                        .lost_focus()
                    || ui
                        .add(
                            egui::Slider::new(&mut conf.block_size, 0..=64)
                                .text("Block size(0 = auto)"),
                        )
                        .lost_focus()
                    || ui.button("Reprocess image").clicked()
                {
                    if self.file.valid {
                        self.img.process();
                        if let Some(img) = &self.img.processed {
                            puffin::profile_scope!("s_load_texture");
                            let color_image = egui::ColorImage::from_rgb(
                                [img.width() as usize, img.height() as usize],
                                img.as_raw().as_slice(),
                            );

                            self.texture = Some(ctx.load_texture(
                                "image",
                                color_image,
                                egui::TextureOptions::default(),
                            ));
                        }
                    }
                }

                //TODO: Add other config options
            });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.hyperlink_to("milk_filter", "https://github.com/horoni/milk_filter");
        ui.label(" powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(". ");
        ui.hyperlink_to("Mirror1", "https://milk.shime.ru");
        ui.label(" and ");
        ui.hyperlink_to("Mirror2", "https://milk0.shime.ru");
    });
}

fn load_save_file(app: &MilkApp, ui: &mut egui::Ui) {
    ui.with_layout(
        egui::Layout::top_down_justified(egui::Align::Center),
        |ui| {
            if ui
                .button(RichText::new("Load").color(Color32::WHITE))
                .clicked()
            {
                let sender = app.file_ch.0.clone();
                let task = rfd::AsyncFileDialog::new().pick_file();

                let ctx = ui.ctx().clone();
                execute(async move {
                    let file = task.await;
                    if let Some(file) = file {
                        let data = file.read().await;
                        let name = file.file_name();
                        let valid = name.ends_with(".png")
                            || name.ends_with(".jpg")
                            || name.ends_with(".jpeg");
                        let _ = sender.send(FileDN::new(name, data, valid));
                        ctx.request_repaint();
                    }
                });
            }

            if ui
                .button(RichText::new("Save").color(Color32::WHITE))
                .clicked()
            {
                let id = crate::smix64::random();
                let task = rfd::AsyncFileDialog::new()
                    .set_file_name(format!("filt_{id:16x}.png"))
                    .save_file();

                if let Some(img) = &app.img.processed {
                    let size = img.width() as usize * img.height() as usize;
                    let mut buf = Vec::with_capacity(size);

                    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                        .unwrap();

                    execute(async move {
                        let file = task.await;
                        if let Some(file) = file {
                            _ = file.write(buf.as_slice()).await;
                        }
                    });
                }
            }
        },
    );
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
