use eframe::egui::{
    self,
    ahash::{HashMap, HashMapExt},
};
use image::AnimationDecoder;
use std::time::Duration;

use crate::lifo;

enum Image {
    Static(egui::ColorImage),
    Animated(Vec<(egui::ColorImage, u64)>),
}

enum LoadCommand {
    Load(String),
}

pub enum Texture {
    Static(egui::TextureHandle),
    Animated(Animated),
}

pub struct Animated {
    pub frames: Vec<(egui::TextureHandle, u64)>,
    pub current: usize,
    pub next_update: Duration,
}

pub struct TexLoader {
    textures: HashMap<String, Option<Texture>>,
    sender: lifo::Sender<LoadCommand>,
    receiver: std::sync::mpsc::Receiver<(String, Image)>,
}

fn get_duration() -> Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
}

impl TexLoader {
    pub fn new(ctx: &egui::Context) -> Self {
        let (sender, cmd_receiver) = lifo::channel();
        let (image_sender, receiver) = std::sync::mpsc::channel();
        let textures = HashMap::new();

        let ctx = ctx.clone();
        std::thread::spawn(move || loop {
            match cmd_receiver.recv() {
                Ok(cmd) => match cmd {
                    LoadCommand::Load(image_path) => eapp_utils::capture_error!(
                        error => log::warn!("error when load image '{image_path}': {error}"),
                        {
                            let content = std::fs::read(&image_path)?;
                            match image::guess_format(&content)? {
                                image::ImageFormat::Gif => {
                                    let frames = (
                                        image_path.clone(),
                                        Image::Animated(
                                            image::codecs::gif::GifDecoder::new(std::io::Cursor::new(content))?
                                                .into_frames()
                                                .collect_frames()?
                                                .into_iter()
                                                .map(|frame| {
                                                    let (num, den) = frame.delay().numer_denom_ms();
                                                    (
                                                        egui::ColorImage::from_rgba_unmultiplied(
                                                            [
                                                                frame.buffer().width() as _,
                                                                frame.buffer().height() as _,
                                                            ],
                                                            frame.buffer(),
                                                        ),
                                                        (num as f32 * 1000.0 / den as f32) as _,
                                                    )
                                                })
                                                .collect(),
                                        ),
                                    );
                                    image_sender.send(frames).unwrap();
                                }
                                // image::ImageFormat::WebP => todo!(),
                                fmt => {
                                    let img = image::load_from_memory_with_format(&content, fmt)?;
                                    let size = [img.width() as _, img.height() as _];
                                    let image_buffer = img.to_rgba8();
                                    let pixels = image_buffer.as_flat_samples();

                                    image_sender
                                        .send((
                                            image_path.clone(),
                                            Image::Static(
                                                egui::ColorImage::from_rgba_unmultiplied(
                                                    size,
                                                    pixels.as_slice(),
                                                ),
                                            ),
                                        ))
                                        .unwrap();
                                }
                            }
                            ctx.request_repaint();
                        }
                    ),
                },
                Err(_) => return,
            }
        });

        Self {
            textures,
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

    pub fn update(&mut self, ctx: &egui::Context, cur_image: Option<&String>) {
        if let Some(cur_img) = cur_image {
            self.load(cur_img);

            if let Some(texture) = self.textures.get_mut(cur_img).unwrap() {
                match texture {
                    Texture::Static(_) => (),
                    Texture::Animated(animation) => {
                        let now = get_duration();
                        if animation.next_update <= now {
                            let delay =
                                Duration::from_micros(animation.frames[animation.current].1);
                            animation.current = (animation.current + 1) % animation.frames.len();
                            animation.next_update = now + delay;
                            ctx.request_repaint_after(delay);
                        } else {
                            ctx.request_repaint_after(animation.next_update - now);
                        }
                    }
                }
            }
        }

        loop {
            use std::sync::mpsc::TryRecvError::*;
            match self.receiver.try_recv() {
                Ok((name, image)) => {
                    if let Some(opt_texture) = self.textures.get_mut(&name) {
                        if opt_texture.is_none() {
                            let options = egui::TextureOptions::default();
                            match image {
                                Image::Static(img) => {
                                    *opt_texture = Some(Texture::Static(
                                        ctx.load_texture(&name, img, options),
                                    ));
                                }
                                Image::Animated(imgs) => {
                                    let current = 0;
                                    let next_update = get_duration();
                                    let frames = imgs
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, (img, delay))| {
                                            (
                                                ctx.load_texture(
                                                    format!("{name}_{i}"),
                                                    img,
                                                    options,
                                                ),
                                                delay,
                                            )
                                        })
                                        .collect();
                                    *opt_texture = Some(Texture::Animated(Animated {
                                        frames,
                                        current,
                                        next_update,
                                    }));
                                }
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
}
