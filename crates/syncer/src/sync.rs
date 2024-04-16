use eframe::egui;
use std::{
    cmp::Ordering,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, TryRecvError},
};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy)]
pub enum ItemCmd {
    Create,
    Replace,
    Keep,
}

pub enum SyncCmd {
    Sync(Vec<Option<SyncItem>>),
    Cancel,
}

pub enum SyncResult {
    Complete((usize, Result<(), String>)),
    Pending((usize, f32)),
    CompleteAll,
}

#[derive(Debug)]
pub struct Item {
    pub filename: String,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub cmd: ItemCmd,
    pub ignore: bool,
    pub progress: f32,
}

impl Item {
    pub fn should_sync(&self) -> bool {
        matches!(self.cmd, ItemCmd::Create | ItemCmd::Replace) && !self.ignore
    }
}

impl Default for Item {
    fn default() -> Self {
        Self {
            filename: Default::default(),
            source_path: Default::default(),
            target_path: Default::default(),
            cmd: ItemCmd::Keep,
            ignore: false,
            progress: 0.0,
        }
    }
}

pub struct SyncItem {
    pub source_path: PathBuf,
    pub target_path: PathBuf,
}

impl From<&Item> for Option<SyncItem> {
    fn from(value: &Item) -> Self {
        if value.should_sync() {
            return Some(SyncItem {
                source_path: value.source_path.clone(),
                target_path: value.target_path.clone(),
            });
        }

        None
    }
}

pub struct Syncer {
    receiver: Receiver<SyncResult>,
    sender: Sender<SyncCmd>,
    synchronizing: bool,
    cancel: bool,
}

impl Syncer {
    pub fn new(ctx: &egui::Context) -> (Self, std::thread::JoinHandle<()>) {
        let (sender, cmd_receiver) = std::sync::mpsc::channel();
        let (result_sender, receiver) = std::sync::mpsc::channel();

        let ctx = ctx.clone();
        let handle = std::thread::spawn(move || loop {
            macro_rules! handle_cancel_or_disconnected {
                (cancel => $cancel_expr:expr, disconnect => $disconnect_expr:expr) => {
                    match cmd_receiver.try_recv() {
                        Ok(SyncCmd::Cancel) => $cancel_expr,
                        Err(err) => match err {
                            TryRecvError::Empty => (),
                            TryRecvError::Disconnected => $disconnect_expr,
                        },
                        _ => unreachable!(),
                    }
                };
                ($expr:expr) => {
                    handle_cancel_or_disconnected!(cancel => $expr, disconnect => $expr);
                };
            }

            let mut buffer = [0u8; 1024 * 1024];

            match cmd_receiver.recv() {
                Ok(SyncCmd::Sync(items)) => {
                    for (i, item) in items.iter().enumerate().filter(|(_, v)| v.is_some()) {
                        handle_cancel_or_disconnected!(cancel => break, disconnect => return);
                        let item = item.as_ref().unwrap();
                        let mut do_sync = || -> Result<bool, Box<dyn std::error::Error>> {
                            let source = item.source_path.as_path();
                            let target = item.target_path.as_path();

                            if let Some(target_dir) = target.parent() {
                                std::fs::create_dir_all(target_dir)?;
                            }

                            let source_meta = source.metadata()?;
                            if source_meta.len() <= 128 * 1024 * 1024 {
                                std::fs::copy(source, target)?;
                            } else {
                                let mut source_file = std::fs::File::open(source)?;
                                let mut target_file = std::fs::File::create(target)?;

                                let mut bytes_read = 0;
                                while let Ok(n) = source_file.read(&mut buffer[..]) {
                                    if n == 0 {
                                        break;
                                    }

                                    handle_cancel_or_disconnected!({
                                        drop(target_file);
                                        let _ = std::fs::remove_file(target);
                                        return Ok(true);
                                    });

                                    bytes_read += n;
                                    target_file.write_all(&buffer[..n])?; // 将读取的数据写入目标文件

                                    ctx.request_repaint();
                                    let _ = result_sender.send(SyncResult::Pending((
                                        i,
                                        bytes_read as f32 / source_meta.len() as f32,
                                    )));
                                }

                                target_file.set_permissions(source_meta.permissions())?;
                                target_file.set_modified(source_meta.modified()?)?;
                            }

                            Ok(false)
                        };

                        let result = match do_sync() {
                            Ok(true) => break,
                            Ok(false) => Ok(()),
                            Err(err) => Err(err.to_string()),
                        };

                        ctx.request_repaint();
                        let _ = result_sender.send(SyncResult::Complete((i, result)));
                    }
                    ctx.request_repaint();
                    let _ = result_sender.send(SyncResult::CompleteAll);
                }
                Ok(SyncCmd::Cancel) => unreachable!(),
                Err(_) => return,
            };
        });

        let synchronizing = false;
        let cancel = false;

        (
            Self {
                receiver,
                sender,
                synchronizing,
                cancel,
            },
            handle,
        )
    }

    pub fn sync(&mut self, items: &[Item]) {
        assert!(!self.synchronizing, "Synchronization has already begun");
        self.sender
            .send(SyncCmd::Sync(Self::to_sync_items(items)))
            .unwrap();
        self.synchronizing = true;
    }

    pub fn cancel(&mut self) {
        assert!(self.synchronizing);
        if !self.cancel {
            self.sender.send(SyncCmd::Cancel).unwrap();
            self.cancel = true
        }
    }

    pub fn update_once(&mut self, items: &mut [Item]) -> Option<Result<bool, String>> {
        if let Ok(result) = self.receiver.try_recv() {
            let mut complete_all = false;
            match result {
                SyncResult::Complete((i, result)) => match result {
                    Ok(_) => items[i].progress = 1.0,
                    Err(err) => return Some(Err(err)),
                },
                SyncResult::Pending((i, progress)) => items[i].progress = progress,
                SyncResult::CompleteAll => {
                    self.synchronizing = false;
                    self.cancel = false;
                    complete_all = true;
                }
            }
            return Some(Ok(complete_all));
        }

        None
    }

    pub fn synchronizing(&self) -> bool {
        self.synchronizing
    }

    fn to_sync_items(items: &[Item]) -> Vec<Option<SyncItem>> {
        items.iter().map(|item| item.into()).collect()
    }
}

pub fn get_items(
    source: &str,
    target: &str,
    items: &mut Vec<Item>,
    only_sync: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    items.clear();

    let source_dir_path = Path::new(source);
    let target_dir_path = Path::new(target);

    for item in WalkDir::new(source_dir_path) {
        let item = item?;
        let source_path = item.path().to_owned();
        if !source_path.is_file() {
            continue;
        }

        let target_path = target_dir_path.join(source_path.strip_prefix(source_dir_path)?);
        if target_path.exists() && !target_path.is_file() {
            return Err(format!(
                "Got same name item, but which is not file '{}'",
                target_path.display()
            )
            .into());
        }

        let source_meta = source_path.metadata()?;
        let source_mod_time = source_meta
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?;
        let filename = source_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let cmd = if target_path.exists() {
            let target_meta = target_path.metadata()?;
            let target_mod_time = Some(
                target_meta
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?,
            );

            match source_mod_time.cmp(target_mod_time.as_ref().unwrap()) {
                Ordering::Less => ItemCmd::Keep,
                Ordering::Equal => {
                    if source_meta.len() != target_meta.len() {
                        return Err(format!(
                            "Files with the same modification time but different sizes: '{}'",
                            target_path.display()
                        )
                        .into());
                    }

                    ItemCmd::Keep
                }
                Ordering::Greater => ItemCmd::Replace,
            }
        } else {
            ItemCmd::Create
        };

        if matches!(cmd, ItemCmd::Keep) && only_sync {
            continue;
        }

        items.push(Item {
            cmd,
            filename,
            source_path,
            target_path,
            ..Default::default()
        });
    }

    Ok(())
}
