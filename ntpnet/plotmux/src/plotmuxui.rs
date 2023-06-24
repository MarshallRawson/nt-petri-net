use crate::plotmux::PlotableData;
use crate::plotpanel::{Panel, PlotPanel};
use crate::plotsource::PlotSource;
use bincode;
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe;
use eframe::egui;
use eframe::egui::widget_text::RichText;
use eframe::egui::widgets::plot;
use eframe::egui::Color32;
use egui_extras::image::RetainedImage;
use image::buffer::ConvertBuffer;
use image::imageops;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use image::DynamicImage::{ImageRgb8, ImageRgba8};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::net::TcpStream;
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use tinyfiledialogs::{open_file_dialog, save_file_dialog};

struct TcpHandler {
    stream: TcpStream,
    sender: Sender<(usize, PlotableData)>,
}
impl TcpHandler {
    fn make(addr: String, tx: Sender<(usize, PlotableData)>) -> Self {
        loop {
            match TcpStream::connect(addr.clone()) {
                Err(e) => {
                    println!("Error while connecting to TCP: {}, trying again!", e);
                    sleep(Duration::from_secs(1));
                }
                Ok(stream) => {
                    return Self {
                        stream: stream,
                        sender: tx,
                    };
                }
            }
        }
    }
    fn spin(mut self, ctx: egui::Context) -> JoinHandle<()> {
        thread::Builder::new()
            .name("plotmuxui-client".into())
            .spawn(move || {
                let mut decoder = snap::raw::Decoder::new();
                loop {
                    let mut len_buf =
                        vec![0; bincode::serialized_size::<usize>(&0_usize).unwrap() as usize];
                    if let Ok(()) = self.stream.read_exact(&mut len_buf) {
                        let mut data_buf = vec![0; bincode::deserialize(&len_buf).unwrap()];
                        if let Err(_) = self.stream.read_exact(&mut data_buf) {
                            continue;
                        }
                        let data_buf = decoder.decompress_vec(&data_buf).unwrap();
                        if let Err(_) = self.sender.send(bincode::deserialize(&data_buf).unwrap()) {
                            continue;
                        }
                        ctx.request_repaint();
                    } else {
                        continue;
                    }
                }
            })
            .expect("unable to spawn plotmuxui-client thread")
    }
}

#[derive(Serialize, Deserialize)]
pub enum PlotMode {
    Text(),
    Series2d(),
    Image(),
}
pub struct PlotMuxUi {
    sources: Vec<Option<PlotSource>>,
    addr: String,
    tcp_threads: Vec<JoinHandle<()>>,
    receiver: Option<(
        Receiver<(usize, PlotableData)>,
        Sender<(usize, PlotableData)>,
    )>,
    graph_image: Option<(RetainedImage, RgbaImage, u32, u32)>,
    font_size: f32,
    root_panel: PlotPanel,
    dialog_thread: Option<JoinHandle<Option<PlotPanel>>>,
}
impl PlotMuxUi {
    pub fn make(graph_png_path: Option<&String>, addr: String) -> Self {
        let graph_image = if let Some(graph_png_path) = graph_png_path {
            let graph_image0 = ImageReader::open(graph_png_path).unwrap();
            let graph_image1 = graph_image0.decode().unwrap();
            let graph_image2: RgbaImage = match graph_image1 {
                ImageRgba8(image) => image,
                ImageRgb8(image) => image.convert(),
                _ => panic!(),
            };
            let w = graph_image2.width();
            let h = graph_image2.height();
            Some((
                RetainedImage::from_color_image(
                    "graph image",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [graph_image2.width() as _, graph_image2.height() as _],
                        graph_image2.as_raw(),
                    ),
                ),
                graph_image2,
                w,
                h,
            ))
        } else {
            None
        };
        PlotMuxUi {
            sources: vec![],
            addr: addr,
            tcp_threads: vec![],
            receiver: None,
            graph_image: graph_image,
            font_size: 15.0,
            root_panel: PlotPanel::new("o".to_owned()),
            dialog_thread: None,
        }
    }
    pub fn spin(mut self) {
        let native_options = eframe::NativeOptions {
            follow_system_theme: true,
            ..eframe::NativeOptions::default()
        };
        let _ = eframe::run_native(
            "PlotMux",
            native_options,
            Box::new(|cc| {
                let (tx, rx) = unbounded();
                self.receiver = Some((rx, tx.clone()));
                self.tcp_threads
                    .push(TcpHandler::make(self.addr.clone(), tx).spin(cc.egui_ctx.clone()));
                Box::new(self)
            }),
        );
    }
}

impl eframe::App for PlotMuxUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok((idx, new_data)) = self.receiver.as_ref().unwrap().0.try_recv() {
            if idx >= self.sources.len() {
                while self.sources.len() < idx + 1 {
                    self.sources.push(None);
                }
            }
            match new_data {
                PlotableData::InitTcp(addr) => {
                    self.tcp_threads.push(
                        TcpHandler::make(addr, self.receiver.as_ref().unwrap().1.clone())
                            .spin(ctx.clone()),
                    );
                }
                PlotableData::InitSource(name) => {
                    self.sources[idx] = Some(PlotSource::make(name));
                }
                _ => self.sources[idx].as_mut().unwrap().new_data(new_data),
            }
        }
        let dialog_thread = self.dialog_thread.take();
        if let Some(dialog_thread) = dialog_thread {
            if dialog_thread.is_finished() {
                if let Ok(panel) = dialog_thread.join() {
                    if let Some(panel) = panel {
                        self.root_panel = panel;
                    }
                }
                self.dialog_thread = None;
            } else {
                self.dialog_thread = Some(dialog_thread);
            }
        }
        let zoom_delta = {
            let mut zoom_delta = 0.0;
            ctx.input(|i| {
                if i.modifiers.shift {
                    self.font_size += (i.zoom_delta() - 1.0) * 2.0;
                } else if i.modifiers.ctrl {
                    zoom_delta = (i.zoom_delta() - 1.0) * 2.0;
                    zoom_delta *= 100.;
                }
                if self.dialog_thread.is_none() {
                    if i.modifiers.ctrl && i.key_released(egui::Key::O) {
                        self.dialog_thread = Some(
                            thread::Builder::new()
                                .name("plotmuxui-open-dialog".into())
                                .spawn(open_layout)
                                .unwrap(),
                        );
                    } else if i.modifiers.ctrl && i.key_released(egui::Key::S) {
                        let bytes = bincode::serialize(&self.root_panel).unwrap();
                        self.dialog_thread = Some(
                            thread::Builder::new()
                                .name("plotmuxui-save-dialog".into())
                                .spawn(move || {
                                    save_layout(&bytes);
                                    None
                                })
                                .unwrap(),
                        );
                    }
                }
            });
            zoom_delta
        };
        if zoom_delta != 0.0 {
            if self.graph_image.is_some() {
                let (_, graph_image, w, h) = self.graph_image.take().unwrap();
                let w = (w as f32 + zoom_delta) as u32;
                let h = (h as f32 + zoom_delta) as u32;
                let graph_image2 = imageops::resize(
                    &DynamicImage::ImageRgba8(graph_image.clone()),
                    w,
                    h,
                    imageops::FilterType::Triangle,
                );
                self.graph_image = Some((
                    RetainedImage::from_color_image(
                        "graph image",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [graph_image2.width() as _, graph_image2.height() as _],
                            graph_image2.as_raw(),
                        ),
                    ),
                    graph_image,
                    w,
                    h,
                ))
            }
        }
        self.font_size = self.font_size.clamp(5.0, 100.0);
        let rich_text = {
            let font = self.font_size;
            move |string: &str| RichText::new(string).size(font)
        };
        let source_mode_select =
            |ui: &mut egui::Ui, show_graph: &mut bool, source_search: &mut String| {
                ui.checkbox(show_graph, rich_text("Graph"));
                if *show_graph {
                    return None;
                }
                let mut ret = None;
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(rich_text("Source: "));
                        ui.text_edit_singleline(source_search);
                    });
                    let possible_source_names = self
                        .sources
                        .iter()
                        .enumerate()
                        .filter_map(|(i, source)| {
                            if let Some(source) = source {
                                if source.name.starts_with(&(*source_search)) {
                                    Some((i, source.name.clone()))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<(usize, String)>>();
                    let buttons = {
                        let mut buttons = vec![];
                        for (i, s) in possible_source_names {
                            ui.horizontal(|ui| {
                                ui.label(rich_text(&s));
                                let text = ui.button(rich_text("text"));
                                let series_2d = ui.button(rich_text("series2d"));
                                let image = ui.button(rich_text("image"));
                                buttons.push((i, text, series_2d, image));
                            });
                        }
                        buttons
                    };
                    for (i, text, series_2d, image) in buttons {
                        if text.clicked() {
                            ret = Some((PlotMode::Text(), i));
                        } else if series_2d.clicked() {
                            ret = Some((PlotMode::Series2d(), i));
                        } else if image.clicked() {
                            ret = Some((PlotMode::Image(), i));
                        }
                    }
                });
                ret
            };
        let graph_image = &self.graph_image;
        let plot_graph = |ui: &mut egui::Ui| {
            if let Some(graph_image) = graph_image {
                egui::ScrollArea::both().show(ui, |ui| {
                    graph_image.0.show(ui);
                });
            }
        };
        let sources = &self.sources;
        let plot_source = |ui: &mut egui::Ui,
                           source_idx: usize,
                           mode: &PlotMode,
                           series_2d_history: &mut f64,
                           plot_height: &mut f32| {
            let mut ret = true;
            ui.horizontal(|ui| {
                ui.heading(rich_text(&sources[source_idx].as_ref().unwrap().name));
                if ui.button(rich_text("exit")).clicked() {
                    ret = false;
                }
            });
            if !ret {
                return false;
            }
            match mode {
                PlotMode::Text() => {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for t in &sources[source_idx].as_ref().unwrap().texts {
                                let text = match t {
                                    (Some(t), text) => {
                                        rich_text(&("[".to_string() + &t.1 + "]: " + &text))
                                            .color(Color32::from_rgb(t.0 .0, t.0 .1, t.0 .2))
                                    }
                                    (None, text) => rich_text(&text),
                                };
                                ui.label(text);
                            }
                        });
                }
                PlotMode::Series2d() => {
                    ui.horizontal(|ui| {
                        ui.label(rich_text("History:"));
                        ui.add(egui::DragValue::new(series_2d_history).speed(1.0));
                        ui.label(rich_text("Plot heights:"));
                        ui.add(egui::Slider::new(plot_height, 10.0..=1000.));
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (plot_name, plot) in
                            &sources[source_idx].as_ref().unwrap().series_plots_2d
                        {
                            ui.label(rich_text(plot_name));
                            plot::Plot::new(plot_name)
                                .height(*plot_height)
                                .legend(plot::Legend::default())
                                .show(ui, |plot_ui| {
                                    for (name, (color, vec)) in plot {
                                        let plot_vec = {
                                            if *series_2d_history <= 0.0 {
                                                *series_2d_history = 0.0;
                                                vec.iter().cloned().collect()
                                            } else if let Some(start) = vec.iter().position(|&v| {
                                                v.x > vec.back().unwrap().x - *series_2d_history
                                            }) {
                                                vec.range(start..).cloned().collect::<Vec<_>>()
                                            } else {
                                                vec![]
                                            }
                                        };
                                        let line =
                                            plot::Line::new(plot::PlotPoints::Owned(plot_vec))
                                                .name(name)
                                                .color(egui::Color32::from_rgb(
                                                    color.0, color.1, color.2,
                                                ));
                                        plot_ui.line(line);
                                    }
                                });
                        }
                    });
                }
                PlotMode::Image() => {
                    egui::ScrollArea::both().show(ui, |ui| {
                        for (image_name, _, plot_image) in
                            &sources[source_idx].as_ref().unwrap().image_plots
                        {
                            ui.label(rich_text(image_name));
                            plot_image.show(ui);
                        }
                    });
                }
            }
            true
        };
        egui::CentralPanel::default().show(ctx, |ui| {
            self.root_panel.show(
                ui,
                Panel::Vertical(
                    egui::TopBottomPanel::top(self.root_panel.name.clone()).resizable(true),
                ),
                &rich_text,
                &source_mode_select,
                &plot_graph,
                &plot_source,
            );
        });
    }
}

fn open_layout() -> Option<PlotPanel> {
    if let Some(layout_file) = open_file_dialog("Open Layout", "~", None) {
        if let Ok(bytes) = fs::read(layout_file) {
            if let Ok(panel) = bincode::deserialize(&bytes) {
                return Some(panel);
            }
        }
    }
    None
}

fn save_layout(bytes: &[u8]) {
    if let Some(layout_file) = save_file_dialog("Save Layout", "~") {
        fs::write(layout_file, bytes).unwrap();
    }
}
