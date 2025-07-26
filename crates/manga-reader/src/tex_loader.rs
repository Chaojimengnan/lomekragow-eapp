use eframe::egui::{
    self,
    ahash::{HashMap, HashMapExt},
};
use image::{
    AnimationDecoder, DynamicImage, Frame,
    codecs::{gif::GifDecoder, webp::WebPDecoder},
};
use std::{
    io::Cursor,
    time::{Duration, Instant},
};

use crate::lifo;

enum Image {
    Static(egui::ColorImage),
    Animated(Vec<(egui::ColorImage, u64)>),
}

enum LoadCommand {
    Load(String),
}

pub enum Texture {
    Static {
        handle: egui::TextureHandle,
        average_color: egui::Color32,
    },
    Animated {
        frames: Vec<(egui::TextureHandle, u64)>,
        current: usize,
        next_update: Instant,
        average_color: egui::Color32,
    },
}

impl Texture {
    pub fn get_cur_handle(&self) -> &egui::TextureHandle {
        match self {
            Self::Static { handle, .. } => handle,
            Self::Animated {
                frames, current, ..
            } => &frames[*current].0,
        }
    }

    pub fn get_cur_average_color(&self) -> egui::Color32 {
        match self {
            Self::Static { average_color, .. } => *average_color,
            Self::Animated { average_color, .. } => *average_color,
        }
    }
}

pub struct TexLoader {
    textures: HashMap<String, Option<Texture>>,
    average_colors: HashMap<String, egui::Color32>,
    sender: lifo::Sender<LoadCommand>,
    receiver: std::sync::mpsc::Receiver<(String, Image)>,
}

fn calculate_average_color(pixels: &[egui::Color32]) -> egui::Color32 {
    if pixels.is_empty() {
        return egui::Color32::TRANSPARENT;
    }

    let step = 8;
    let (mut r, mut g, mut b, mut count) = (0u32, 0u32, 0u32, 0u32);

    for i in (0..pixels.len()).step_by(step) {
        let pixel = pixels[i];
        r += pixel.r() as u32;
        g += pixel.g() as u32;
        b += pixel.b() as u32;
        count += 1;
    }

    if count == 0 {
        egui::Color32::TRANSPARENT
    } else {
        egui::Color32::from_rgb((r / count) as u8, (g / count) as u8, (b / count) as u8)
    }
}

impl TexLoader {
    pub fn new(ctx: &egui::Context) -> Self {
        let (sender, cmd_receiver) = lifo::channel();
        let (image_sender, receiver) = std::sync::mpsc::channel();
        let textures = HashMap::new();
        let average_colors = HashMap::new();

        let ctx = ctx.clone();
        std::thread::spawn(move || {
            loop {
                let Ok(cmd) = cmd_receiver.recv() else {
                    return;
                };

                match cmd {
                    LoadCommand::Load(image_path) => {
                        let image = match Self::load_image(&image_path) {
                            Ok(image) => image,
                            Err(error) => {
                                log::warn!("error when load image '{image_path}': {error}");
                                continue;
                            }
                        };

                        image_sender.send((image_path, image)).unwrap();
                        ctx.request_repaint();
                    }
                };
            }
        });

        Self {
            textures,
            average_colors,
            sender,
            receiver,
        }
    }

    pub fn load(&mut self, image_path: &str) {
        if !self.textures.contains_key(image_path) {
            self.textures.insert(image_path.to_owned(), None);
            self.sender
                .send(LoadCommand::Load(image_path.to_owned()))
                .unwrap();
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, cur_image: Option<&str>) {
        if let Some(cur_img) = cur_image {
            self.load(cur_img);

            if let Some(texture) = self.textures.get_mut(cur_img).unwrap() {
                match texture {
                    Texture::Static { .. } => (),
                    Texture::Animated {
                        frames,
                        current,
                        next_update,
                        ..
                    } => {
                        let now = Instant::now();
                        if now >= *next_update {
                            let delay = Duration::from_millis(frames[*current].1);
                            *current = (*current + 1) % frames.len();
                            *next_update = now + delay;
                            let remaining = *next_update - now;
                            ctx.request_repaint_after(remaining);
                        } else {
                            let remaining = *next_update - now;
                            ctx.request_repaint_after(remaining);
                        }
                    }
                }
            }
        }

        loop {
            use std::sync::mpsc::TryRecvError::*;
            let options = egui::TextureOptions::default();

            macro_rules! get_or_calculate_average_color {
                ($path:expr, $pixels:expr) => {
                    match self.average_colors.get($path) {
                        Some(c) => *c,
                        None => {
                            let color = calculate_average_color($pixels);
                            self.average_colors.insert($path.to_owned(), color);
                            color
                        }
                    }
                };
            }

            match self.receiver.try_recv() {
                Ok((image_path, image)) => {
                    if let Some(opt_texture) = self.textures.get_mut(&image_path) {
                        if opt_texture.is_some() {
                            continue;
                        }

                        match image {
                            Image::Static(img) => {
                                let average_color =
                                    get_or_calculate_average_color!(&image_path, &img.pixels);

                                *opt_texture = Some(Texture::Static {
                                    handle: ctx.load_texture(&image_path, img, options),
                                    average_color,
                                });
                            }
                            Image::Animated(imgs) => {
                                let current = 0;
                                let next_update = Instant::now();
                                let average_color = if let Some(first_frame) = imgs.first() {
                                    get_or_calculate_average_color!(
                                        &image_path,
                                        &first_frame.0.pixels
                                    )
                                } else {
                                    egui::Color32::TRANSPARENT
                                };

                                let frames = imgs
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, (img, delay))| {
                                        (
                                            ctx.load_texture(
                                                format!("{image_path}_{i}"),
                                                img,
                                                options,
                                            ),
                                            delay,
                                        )
                                    })
                                    .collect();

                                *opt_texture = Some(Texture::Animated {
                                    frames,
                                    current,
                                    next_update,
                                    average_color,
                                });
                            }
                        }
                    }
                }
                Err(err) => match err {
                    Empty => break,
                    Disconnected => unreachable!(),
                },
            };
        }
    }

    pub fn textures(&self) -> &HashMap<String, Option<Texture>> {
        &self.textures
    }

    pub fn forget_all(&mut self) {
        self.textures.clear();
    }

    fn dynamic_image_to_image(img: DynamicImage) -> Image {
        let size = [img.width() as _, img.height() as _];
        let image_buffer = img.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        Image::Static(color_image)
    }

    fn frames_to_image(frames: Vec<Frame>) -> Image {
        let frames = frames
            .into_iter()
            .map(|frame| {
                let (num, den) = frame.delay().numer_denom_ms();
                let delay_ms = num as f32 / den as f32;
                (
                    egui::ColorImage::from_rgba_unmultiplied(
                        [frame.buffer().width() as _, frame.buffer().height() as _],
                        frame.buffer(),
                    ),
                    delay_ms as u64,
                )
            })
            .collect();

        Image::Animated(frames)
    }

    fn load_image(image_path: &str) -> Result<Image, Box<dyn std::error::Error>> {
        let content = std::fs::read(image_path)?;
        let image = match image::guess_format(&content)? {
            image::ImageFormat::Gif => Self::frames_to_image(
                GifDecoder::new(Cursor::new(content))?
                    .into_frames()
                    .collect_frames()?,
            ),
            image::ImageFormat::WebP => {
                let decoder = WebPDecoder::new(Cursor::new(&content))?;
                if decoder.has_animation() {
                    Self::frames_to_image(decoder.into_frames().collect_frames()?)
                } else {
                    Self::dynamic_image_to_image(DynamicImage::from_decoder(decoder)?)
                }
            }
            fmt => {
                Self::dynamic_image_to_image(image::load_from_memory_with_format(&content, fmt)?)
            }
        };

        Ok(image)
    }
}
