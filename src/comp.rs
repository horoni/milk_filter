use image::Rgb;
use rayon::prelude::*;

// Simulates quantization by reducing color precision across pixels in parallel
pub fn jpeg_quantization(img: &mut image::ImageBuffer<Rgb<u8>, Vec<u8>>, quality_factor: f32) {
    puffin::profile_function!();

    let num_levels_f32 = 2.0 + (254.0 * quality_factor.clamp(0.0, 1.0));
    let num_levels = num_levels_f32 as u32;

    let num_levels = num_levels.max(2);
    let num_levels_m1 = num_levels - 1;

    let mut lut = [0u8; 256];

    for input_val in 0..=255 {
        let level_index = (input_val as u32 * num_levels) / 256;
        let output_val = (level_index * 255) / num_levels_m1;
        lut[input_val as usize] = output_val.min(255) as u8;
    }

    img.as_mut().par_iter_mut().for_each(|byte| {
        *byte = lut[*byte as usize];
    });
}

// Simulates blockiness by averaging colors within blocks across rows in parallel
pub fn jpeg_blockiness(img: &mut image::ImageBuffer<Rgb<u8>, Vec<u8>>, block_size: u32) {
    puffin::profile_function!();

    if block_size <= 1 {
        return;
    }

    let width = img.width() as usize;
    let block_size = block_size as usize;
    let step = block_size * 3;

    img.as_mut()
        .par_chunks_exact_mut(width * 3)
        .for_each(|row_slice| {
            let mut chunks = row_slice.chunks_exact_mut(step);

            for block in chunks.by_ref() {
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;

                for pixel in block.chunks_exact(3) {
                    sum_r += pixel[0] as u32;
                    sum_g += pixel[1] as u32;
                    sum_b += pixel[2] as u32;
                }

                let avg_r = (sum_r / block_size as u32) as u8;
                let avg_g = (sum_g / block_size as u32) as u8;
                let avg_b = (sum_b / block_size as u32) as u8;

                for pixel in block.chunks_exact_mut(3) {
                    pixel[0] = avg_r;
                    pixel[1] = avg_g;
                    pixel[2] = avg_b;
                }
            }

            let tail = chunks.into_remainder();
            if !tail.is_empty() {
                let len_bytes = tail.len();
                let pixel_count = (len_bytes / 3) as u32;

                if pixel_count > 0 {
                    let mut sum_r = 0u32;
                    let mut sum_g = 0u32;
                    let mut sum_b = 0u32;

                    for pixel in tail.chunks_exact(3) {
                        sum_r += pixel[0] as u32;
                        sum_g += pixel[1] as u32;
                        sum_b += pixel[2] as u32;
                    }

                    let avg_r = (sum_r / pixel_count) as u8;
                    let avg_g = (sum_g / pixel_count) as u8;
                    let avg_b = (sum_b / pixel_count) as u8;

                    for pixel in tail.chunks_exact_mut(3) {
                        pixel[0] = avg_r;
                        pixel[1] = avg_g;
                        pixel[2] = avg_b;
                    }
                }
            }
        });
}
