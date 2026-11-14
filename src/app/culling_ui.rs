use super::ImageViewerApp;
use crate::culling::{self, Selection, StarFilter, ViewFormat};
use eframe::egui::{self, Key};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

enum ExportEvent {
    Progress(usize, usize),
    Done(Result<culling::ExportReport, String>),
}
#[derive(Default)]
pub(crate) struct CullingUi {
    pub selection: Selection,
    dialog: bool,
    paired: bool,
    root: String,
    receiver: Option<Receiver<ExportEvent>>,
    progress: (usize, usize),
    message: String,
}
impl ImageViewerApp {
    pub(crate) fn culling_pair_policy(
        &self,
        original: crate::settings::PairedJpegHandling,
    ) -> crate::settings::PairedJpegHandling {
        if self.culling.selection.format == ViewFormat::All {
            original
        } else {
            crate::settings::PairedJpegHandling::ShowBoth
        }
    }
    fn reload_culling(&mut self) {
        if let Some(dir) = self.current_browse_directory() {
            self.initial_image = self.image_files.get(self.current_index).cloned();
            self.reload_current_browse_directory(dir);
        }
    }
    fn rate_current(&mut self, stars: u8) {
        if self.scanning {
            return;
        }
        if let Some(path) = self.image_files.get(self.current_index).cloned() {
            self.culling.selection.rate(&path, stars);
            if !self.culling.selection.accepts(&path) {
                self.reload_culling();
            }
        }
    }
    pub(crate) fn handle_culling_keys(&mut self, ctx: &egui::Context) -> bool {
        if self.culling.dialog {
            return true;
        }
        if ctx.egui_wants_keyboard_input() || ctx.input(|i| i.modifiers != egui::Modifiers::NONE) {
            return false;
        }
        for (stars, key) in [
            Key::Num0,
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
        ]
        .into_iter()
        .enumerate()
        {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                self.rate_current(stars as u8);
                return true;
            }
        }
        false
    }
    pub(crate) fn draw_culling_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let mut finished = false;
        if let Some(rx) = &self.culling.receiver {
            loop {
                match rx.try_recv() {
                    Ok(ExportEvent::Progress(done, total)) => self.culling.progress = (done, total),
                    Ok(ExportEvent::Done(result)) => {
                        self.culling.message = match result {
                            Ok(r) => format!(
                                "已复制 {} 个文件；缺失配对 {}；失败 {}\n{}\n{}",
                                r.copied,
                                r.missing_pairs,
                                r.errors.len(),
                                r.directories
                                    .iter()
                                    .map(|p| p.display().to_string())
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                                r.errors.join("\n")
                            ),
                            Err(e) => format!("导出失败：{e}"),
                        };
                        finished = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.culling.message = "导出线程意外结束，请检查目标目录".into();
                        finished = true;
                        break;
                    }
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if finished {
            self.culling.receiver = None;
        }
        let old_format = self.culling.selection.format;
        let old_filter = self.culling.selection.filter.clone();
        let mut rating = None;
        egui::Panel::top("photo_culling").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("选片");
                ui.selectable_value(
                    &mut self.culling.selection.format,
                    ViewFormat::Jpeg,
                    "仅 JPEG",
                );
                ui.selectable_value(
                    &mut self.culling.selection.format,
                    ViewFormat::Raw,
                    "仅 RAW",
                );
                ui.selectable_value(
                    &mut self.culling.selection.format,
                    ViewFormat::All,
                    "全部格式",
                );
                ui.separator();
                let filter = &mut self.culling.selection.filter;
                egui::ComboBox::from_id_salt("star_filter")
                    .selected_text(match filter {
                        StarFilter::AtLeast(_) => "大于等于",
                        StarFilter::Exactly(_) => "等于",
                        StarFilter::AnyOf(_) => "组合",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(filter, StarFilter::AtLeast(0), "大于等于");
                        ui.selectable_value(filter, StarFilter::Exactly(4), "等于");
                        ui.selectable_value(filter, StarFilter::AnyOf([true; 6]), "组合");
                    });
                match filter {
                    StarFilter::AtLeast(n) | StarFilter::Exactly(n) => {
                        ui.add(egui::Slider::new(n, 0..=5).text("星"));
                    }
                    StarFilter::AnyOf(mask) => {
                        for (n, enabled) in mask.iter_mut().enumerate() {
                            ui.checkbox(enabled, format!("{n}星"));
                        }
                    }
                }
                if let Some(path) = self.image_files.get(self.current_index) {
                    let stars = self.culling.selection.rating(path);
                    ui.label(format!(
                        "当前：{}{}",
                        "★".repeat(stars as usize),
                        "☆".repeat(5 - stars as usize)
                    ));
                    for n in 0..=5 {
                        if ui
                            .small_button(n.to_string())
                            .on_hover_text("评分（当前会话保存）")
                            .clicked()
                        {
                            rating = Some(n);
                        }
                    }
                }
                if ui
                    .add_enabled(
                        !self.scanning
                            && !self.image_files.is_empty()
                            && self.culling.receiver.is_none(),
                        egui::Button::new("导出筛选结果…"),
                    )
                    .clicked()
                {
                    self.culling.dialog = true;
                    self.culling.root.clear();
                }
                if self.culling.receiver.is_some() {
                    let (done, total) = self.culling.progress;
                    ui.add(
                        egui::ProgressBar::new(if total == 0 {
                            0.0
                        } else {
                            done as f32 / total as f32
                        })
                        .text(format!("导出 {done}/{total}")),
                    );
                }
            });
            if self.image_files.is_empty() && !self.scanning {
                ui.label("没有符合条件的图片，可切换格式或星级筛选。");
            }
            if !self.culling.message.is_empty() {
                ui.collapsing("导出结果", |ui| {
                    ui.label(&self.culling.message);
                });
            }
        });
        if old_format != self.culling.selection.format
            || old_filter != self.culling.selection.filter
        {
            self.reload_culling();
        }
        if let Some(stars) = rating {
            self.rate_current(stars);
        }
        if self.culling.dialog {
            let mut open = true;
            let mut start = false;
            egui::Window::new("导出 JPEG / RAW").open(&mut open).collapsible(false).show(&ctx, |ui| {
                ui.label(format!("导出当前筛选列表中的 JPEG / RAW（共 {} 项）", self.image_files.iter().filter(|p| culling::is_jpeg(p) || culling::is_raw(p)).count()));
                ui.radio_value(&mut self.culling.paired, false, "仅导出当前列表中的文件");
                ui.radio_value(&mut self.culling.paired, true, "同时导出同目录、同名的另一格式");
                ui.label("自定义根目录（留空使用当前浏览目录）：");
                ui.text_edit_singleline(&mut self.culling.root);
                ui.label("复制到 jpgHHMM / rawHHMM；已有目录自动追加序号，不覆盖文件。\n评分保存在当前会话，关闭程序后清除。");
                start = ui.add_enabled(!self.scanning, egui::Button::new("开始导出")).clicked();
            });
            self.culling.dialog = open;
            if start {
                self.culling.dialog = false;
                self.start_culling_export(&ctx);
            }
        }
    }
    fn start_culling_export(&mut self, ctx: &egui::Context) {
        let root = if self.culling.root.trim().is_empty() {
            self.current_browse_directory()
        } else {
            Some(PathBuf::from(self.culling.root.trim()))
        };
        let Some(root) = root else {
            return;
        };
        let sources = self.image_files.clone();
        let paired = self.culling.paired;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let local = crate::ui::osd::format_file_modified(now);
        let stamp = local.get(11..16).unwrap_or("").replace(':', "");
        let (tx, rx) = mpsc::channel();
        self.culling.receiver = Some(rx);
        self.culling.progress = (0, 0);
        self.culling.message.clear();
        let ctx = ctx.clone();
        if !self.background_threads.spawn("photo-export", move || {
            let result = culling::export(&sources, &root, &stamp, paired, |done, total| {
                let _ = tx.send(ExportEvent::Progress(done, total));
                ctx.request_repaint();
            })
            .map_err(|e| e.to_string());
            let _ = tx.send(ExportEvent::Done(result));
            ctx.request_repaint();
        }) {
            self.culling.receiver = None;
            self.culling.message = "无法启动导出线程".into();
        }
    }
}
