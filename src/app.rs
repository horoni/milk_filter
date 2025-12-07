use crate::filt::MilkImage;
use egui::{Color32, RichText};
use std::future::Future;
use std::sync::mpsc::{channel, Sender, Receiver};


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
                            img.as_raw().as_slice()
                    );
                    
                    self.texture = Some(ctx.load_texture(
                        "image", color_image,
                        egui::TextureOptions::default()
                    ));
                } else {
                    self.texture = None;
                }
            }
        }

        egui::Window::new("About")
            .open(&mut self.show_about)
            .vscroll(true)
            .collapsible(false)
            .show(ctx, |ui| {
                //egui::TopBottomPanel::top("about_top").show_inside(ui, |ui| {
                //});
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.heading(RichText::new("Changelog").color(Color32::WHITE));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("7 Dec. 2025 -> ").color(Color32::YELLOW));
                    ui.label(RichText::new("Init").color(Color32::GREEN));
                });
        });

        egui::Window::new("Config")
            .open(&mut self.show_config)
            .default_size(egui::vec2(200.0, 200.0))
            .vscroll(true)
            .collapsible(true)
            .show(ctx, |ui| {
            let conf = self.img.get_config();
            
            if ui.checkbox(&mut conf.alt, "Alternative pallete").changed()
                || ui.checkbox(&mut conf.pointism, "Pointillism").changed()
                || ui.checkbox(&mut conf.enabled, "Milk Enabled").changed()
                || ui.checkbox(&mut conf.quant, "Quant").changed()
                || ui.checkbox(&mut conf.block, "Blocks").changed()
                || ui.add(egui::Slider::new(&mut conf.comp, 0..=100)
                    .text("(Slow) Compression")
                    .suffix(" %")).lost_focus()
                || ui.add(egui::Slider::new(&mut conf.block_size, 0..=64)
                    .text("Block size(0 = auto)")).lost_focus()
                || ui.button("Reprocess image").clicked()
            {
                if self.file.valid {
                    self.img.process();
                    if let Some(img) = &self.img.processed {
                        puffin::profile_scope!("s_load_texture");
                        let color_image = egui::ColorImage::from_rgb(
                                [img.width() as usize, img.height() as usize],
                                img.as_raw().as_slice()
                        );
                        
                        self.texture = Some(ctx.load_texture(
                            "image", color_image,
                            egui::TextureOptions::default()
                        ));
                    }
                }
            }

            //TODO: Add other config options
        });

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
            ui.heading("Milk-Filter");
            
            load_save_file(&self, ui);
            
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
                ui.add(egui::Image::new(tex)
                    .max_size(max_size));
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
        ui.label(".");
    });
}

fn load_save_file(app: &MilkApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("Load").clicked() {
            let sender = app.file_ch.0.clone();
            let task = rfd::AsyncFileDialog::new().pick_file();

            let ctx = ui.ctx().clone();
            execute(async move {
                let file = task.await;
                if let Some(file) = file {
                    let data = file.read().await;
                    let name = file.file_name();
                    let valid = if name.ends_with(".png")
                        || name.ends_with(".jpg")
                        || name.ends_with(".jpeg") { true } else { false };
                    let _ = sender.send(FileDN::new(name, data, valid));
                    ctx.request_repaint();
                }
            });
        }

        if ui.button("Save").clicked() {
            let task = rfd::AsyncFileDialog::new()
                .set_file_name(format!("filt.png")) // TODO: fill name with random bytes
                .save_file();

            let contents = if let Some(img) = &app.img.processed {
                let size = img.width() as usize * img.height() as usize;
                let mut buf = Vec::with_capacity(size);

                img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .unwrap();

                buf
            } else { panic!("image is not processed yet")};


            execute(async move {
                let file = task.await;
                if let Some(file) = file {
                    _ = file.write(contents.as_slice()).await;
                }
            });
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
