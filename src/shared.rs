extern crate image;

use self::image::{DynamicImage, FilterType, GenericImageView, ImageBuffer, imageops};
use node::{Buffer, ChannelPixel, DetachedBuffer, ResizePolicy, Size};
use std::cmp::max;

pub fn channels_to_rgba(channels: &[&Buffer]) -> Vec<u8> {
    if channels.len() != 4 {
        panic!("The number of channels when converting to an RGBA image needs to be 4");
    }

    channels[0]
        .pixels()
        .zip(channels[1].pixels())
        .zip(channels[2].pixels())
        .zip(channels[3].pixels())
        .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
        .flatten()
        .map(|x| (x[0] * 255.).min(255.) as u8)
        .collect()
}

pub fn deconstruct_image(image: &DynamicImage) -> Vec<Buffer> {
    let raw_pixels = image.raw_pixels();
    let (width, height) = (image.width(), image.height());
    let pixel_count = (width * height) as usize;
    let channel_count = raw_pixels.len() / pixel_count;
    let max_channel_count = 4;
    let mut pixel_vecs: Vec<Vec<f32>> = Vec::with_capacity(max_channel_count);

    for _ in 0..max_channel_count {
        pixel_vecs.push(Vec::with_capacity(pixel_count));
    }

    let mut current_channel = 0;

    for component in raw_pixels {
        pixel_vecs[current_channel].push(ChannelPixel::from(component) / 255.);
        current_channel = (current_channel + 1) % channel_count;
    }

    for (i, mut item) in pixel_vecs
        .iter_mut()
        .enumerate()
        .take(max_channel_count)
        .skip(channel_count)
    {
        *item = match i {
            3 => vec![1.; pixel_count],
            _ => vec![0.; pixel_count],
        }
    }

    pixel_vecs
        .into_iter()
        .map(|p_vec| ImageBuffer::from_raw(width, height, p_vec).unwrap())
        .collect()
}

pub fn resize_buffers(
    buffers: &mut [DetachedBuffer],
    policy: Option<ResizePolicy>,
    filter: Option<FilterType>,
) {
    if buffers.len() < 2 {
        return;
    }

    let policy = policy.unwrap_or(ResizePolicy::MostPixels);
    let filter = filter.unwrap_or(FilterType::CatmullRom);

    let size = match policy {
        ResizePolicy::MostPixels => buffers
            .iter()
            .max_by(|a, b| a.size().pixel_count().cmp(&b.size().pixel_count()))
            .map(|buffer| buffer.size())
            .unwrap(),
        ResizePolicy::LeastPixels => buffers
            .iter()
            .min_by(|a, b| a.size().pixel_count().cmp(&b.size().pixel_count()))
            .map(|buffer| buffer.size())
            .unwrap(),
        ResizePolicy::LargestAxes => buffers
            .iter()
            .fold(Size::new(0, 0), |a, b| {
                Size::new(
                    max(a.width(), b.size().width()),
                    max(a.height(), b.size().height()),
                )
            }),
        _ => unimplemented!(),
    };

    buffers
        .iter_mut()
        .filter(|ref buffer| buffer.size() != size)
        .for_each(|ref mut buffer| {
            let resized_buffer = imageops::resize(
                &*buffer.buffer(),
                size.width(),
                size.height(),
                filter,
            );
            buffer.set_buffer(resized_buffer);
            buffer.set_size(size);
        });
}