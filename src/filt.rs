use image::Rgb;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use rayon::prelude::*;

fn probably(rng: &mut SmallRng, chance: f32) -> bool {
    rng.random_range(0.0..1.0) < chance
}

pub struct MilkImage {
    img: Option<image::ImageBuffer<Rgb<u8>, Vec<u8>>>,
    pub processed: Option<image::ImageBuffer<Rgb<u8>, Vec<u8>>>,
    conf: MilkConfig,
}

impl MilkImage {
    pub fn new() -> Self {
        Self {
            img: None,
            processed: None,
            conf: MilkConfig::new(),
        }
    }
    
    pub fn open(&mut self, data: &[u8]) {
        puffin::profile_function!();
        
        let img = {
            puffin::profile_scope!("s_load_from_mem");
            match image::load_from_memory(data) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("failed to open image: {e}");
                    std::process::exit(1);
                }
            }
        };

        let img = {
            puffin::profile_scope!("s_convert_to_rgb8");
            img.into_rgb8()
        };
        self.img = Some(img);
    }

    pub fn process(&mut self) {
        puffin::profile_function!();

        let mut img = {
            puffin::profile_scope!("s_clone_img");
            self.img.clone().unwrap()
        };

        let mut img = if self.conf.comp > 0 {
            puffin::profile_scope!("s_compress");
/*
            let estimated_size = img.width() as usize * img.height() as usize;
            let mut comp_buf = Vec::with_capacity(estimated_size);

            let quality = std::cmp::max(1, 100 - self.conf.comp);
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut comp_buf, quality);

            if let Err(e) = img.write_with_encoder(enc) {
                eprintln!("failed to compress image: {e}");
                std::process::exit(1);
            }

            image::load_from_memory(&comp_buf)
                .expect("failed to load compressed image")
                .into_rgb8()
*/
            let quality_factor = (100.0 - self.conf.comp as f32) / 100.0;
            let block_size = if self.conf.block_size == 0 {
                (((self.conf.comp as f32 / 100.0) * 7.0).max(1.0) as u32).min(8)
            } else { self.conf.block_size };


            if self.conf.quant {
                crate::comp::jpeg_quantization(&mut img, quality_factor.max(0.05));
            }
            if self.conf.block {
                crate::comp::jpeg_blockiness(&mut img, block_size);
            }
            img
        } else {
            img
        };

        let (width, _) = img.dimensions();
        let width = width as usize;
        let raw_pixels = img.as_mut();

        let color_map = [
            [(0u8, 0u8, 0u8), (102u8, 0u8, 31u8), (137u8, 0u8, 146u8)],
            [(0u8, 0u8, 0u8), (92u8, 36u8, 60u8), (203u8, 43u8, 43u8)],
        ];
        let colors = if self.conf.alt { color_map[1] } else { color_map[0] };
        let chance = if self.conf.pointism { 0.7 } else { 1.0 };
        let (thr_mid1, thr_mid2) = if self.conf.alt { (90u16, 150u16) } else { (120u16, 200u16) };

        if self.conf.enabled
        {
        puffin::profile_scope!("s_apply_filter");
        raw_pixels
            .par_chunks_mut(width * 3)
            .enumerate()
            .for_each(|(y, row)| {
                let seed = ((width * 3) + y) as u64 ^ 0x123456789abcdef0;
                let mut rng = SmallRng::seed_from_u64(seed);

                for pixel in row.chunks_exact_mut(3) {
                    let r = pixel[0] as u16;
                    let g = pixel[1] as u16;
                    let b = pixel[2] as u16;

                    let bright = (r + g + b) / 3;

                    let color = if bright <= 25 {
                        if let Some(i) = self.conf.s1 {
                            colors[i]
                        } else {
                            colors[0]
                        }
                    } else if bright <= 70 {
                        if let Some(i) = self.conf.s2 {
                            colors[i]
                        } else {
                            if self.conf.eff == 1 {
                                if probably(&mut rng, chance) { colors[1] } else { colors[0] }
                            } else {
                                if probably(&mut rng, chance) { colors[0] } else { colors[1] }
                            }
                        }
                    } else if bright < thr_mid1 {
                        if let Some(i) = self.conf.s3 {
                            colors[i]
                        } else {
                            if self.conf.eff == 1 {
                                colors[0]
                            } else {
                                if probably(&mut rng, chance) { colors[1] } else { colors[0] }
                            }
                        }
                    } else if bright < thr_mid2  {
                        if let Some(i) = self.conf.s4 {
                            colors[i]
                        } else {
                            if self.conf.eff == 1 {
                                if probably(&mut rng, chance) { colors[0] } else { colors[1] }
                            } else {
                                colors[1]
                            }
                        }
                    } else if bright < 230 {
                        if let Some(i) = self.conf.s5 {
                            colors[i]
                        } else {
                            if self.conf.eff == 1 {
                                colors[2]
                            } else {
                                if probably(&mut rng, chance) { colors[2] } else { colors[1] }
                            }
                        }
                    } else {
                        if let Some(i) = self.conf.s6 {
                            colors[i]
                        } else {
                            colors[2]
                        }
                    };

                    pixel[0] = color.0;
                    pixel[1] = color.1;
                    pixel[2] = color.2;
                }
            });
        }

        self.processed = Some(img);
    }

    pub fn get_config(&mut self) -> &mut MilkConfig {
        &mut self.conf
    }
}

pub struct MilkConfig {
    pub alt: bool,
    pub pointism: bool,
    pub comp: u8,

    pub enabled: bool,
    pub quant: bool,
    pub block: bool,
    pub block_size: u32,

    pub eff: u8,
    pub s1: Option<usize>,
    pub s2: Option<usize>,
    pub s3: Option<usize>,
    pub s4: Option<usize>,
    pub s5: Option<usize>,
    pub s6: Option<usize>,
}

impl MilkConfig {
    fn new() -> Self {
        Self {
            alt: false,
            pointism: false,
            comp: 0,
            enabled: true,
            quant: true,
            block: true,
            block_size: 0,
            eff: 0,
            s1: None,
            s2: None,
            s3: None,
            s4: None,
            s5: None,
            s6: None,
        }
    }
}
