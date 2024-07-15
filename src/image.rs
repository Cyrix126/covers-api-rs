use std::{fs::File, path::Path};

use anyhow::Result;
use image::{imageops::FilterType, load_from_memory};
pub const RES_COVER_MINI: u32 = 45;
pub const RES_COVER_SMALL: u32 = 135;
pub const RES_COVER_ORIGIN: u32 = 240;
pub fn write_cover(cover: &[u8], id: u32, path_cover: &Path) -> Result<()> {
    let image = load_from_memory(cover)?;
    // resize image
    let filter_resize = FilterType::Lanczos3;
    let cover_mini = image.resize(RES_COVER_MINI, RES_COVER_MINI, filter_resize);
    let cover_small = image.resize(RES_COVER_SMALL, RES_COVER_SMALL, filter_resize);
    let cover_origin = image.resize(RES_COVER_ORIGIN, RES_COVER_ORIGIN, filter_resize);

    // write into path files

    // write into mini image file
    let mut path_mini = path_cover.to_path_buf();
    path_mini.push(format!("{id}-M.webp"));
    let mut file = File::create_new(path_mini)?;
    cover_mini.write_to(&mut file, image::ImageFormat::WebP)?;

    // write into small image file
    let mut path_small = path_cover.to_path_buf();
    path_small.push(format!("{id}-S.webp"));
    let mut file = File::create_new(path_small)?;
    cover_small.write_to(&mut file, image::ImageFormat::WebP)?;

    // write into large image file
    let mut path_origin = path_cover.to_path_buf();
    path_origin.push(format!("{id}-L.webp"));
    let mut file = File::create_new(path_origin)?;
    cover_origin.write_to(&mut file, image::ImageFormat::WebP)?;

    Ok(())
}
