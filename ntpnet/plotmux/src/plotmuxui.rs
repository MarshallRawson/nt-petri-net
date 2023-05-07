use crate::plotmux::PlotableData;
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
use image::io::Reader as ImageReader;
use image::DynamicImage::{ImageRgb8, ImageRgba8};
use image::RgbaImage;
use std::io::Read;
use std::net::TcpStream;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

struct TcpHandler {
    stream: TcpStream,
    sender: Sender<(usize, PlotableData)>,
}
impl TcpHandler {
    fn make(addr: String) -> (Self, Receiver<(usize, PlotableData)>) {
        loop {
            match TcpStream::connect(addr.clone()) {
                Err(e) => {
                    println!("Error while connecting to TCP: {}, trying again!", e);
                    sleep(Duration::from_secs(1));
                }
                Ok(stream) => {
                    let (tx, rx) = unbounded();
                    return (
                        Self {
                            stream: stream,
                            sender: tx,
                        },
                        rx,
                    );
                }
            }
        }
    }
    fn spin(mut self, ctx: egui::Context) {
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
            .expect("unable to spawn plotmuxui-client thread");
    }
}

enum PlotMode {
    Text(),
    Series2d(),
    Image(),
}
pub struct PlotMuxUi {
    sources: Vec<Option<PlotSource>>,
    addr: String,
    receiver: Option<Receiver<(usize, PlotableData)>>,
    source_search: String,
    graph_image: Option<RetainedImage>,
    show_graph: bool,
    selected_source: Option<usize>,
    mode: Option<PlotMode>,
    series_2d_history: f64,
    font_size: f32,
    plot_ratios: f32,
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
            Some(RetainedImage::from_color_image(
                "graph image",
                egui::ColorImage::from_rgba_unmultiplied(
                    [graph_image2.width() as _, graph_image2.height() as _],
                    graph_image2.as_raw(),
                ),
            ))
        } else {
            None
        };
        PlotMuxUi {
            sources: vec![],
            addr: addr,
            receiver: None,
            source_search: "".into(),
            graph_image: graph_image,
            show_graph: false,
            selected_source: None,
            mode: None,
            series_2d_history: 0.0,
            font_size: 15.0,
            plot_ratios: 4.0,
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
                let (tcp_handler, rx) = TcpHandler::make(self.addr.clone());
                self.receiver = Some(rx);
                tcp_handler.spin(cc.egui_ctx.clone());
                Box::new(self)
            }),
        );
    }
}

impl eframe::App for PlotMuxUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok((idx, new_data)) = self.receiver.as_ref().unwrap().try_recv() {
            if idx >= self.sources.len() {
                while self.sources.len() < idx + 1 {
                    self.sources.push(None);
                }
            }
            match new_data {
                PlotableData::InitSource(name) => {
                    self.sources[idx] = Some(PlotSource::make(name));
                }
                _ => self.sources[idx].as_mut().unwrap().new_data(new_data),
            }
        }
        ctx.input(|i| {
            if i.modifiers.shift {
                self.font_size += (i.zoom_delta() - 1.0) * 2.0;
            }
        });
        self.font_size = self.font_size.clamp(5.0, 100.0);
        let rich_text = {
            let font = self.font_size;
            move |string: &str| RichText::new(string).size(font)
        };
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.checkbox(&mut self.show_graph, rich_text("Graph"));
            if self.show_graph {
                if let Some(graph_image) = &self.graph_image {
                    graph_image.show(ui);
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(rich_text("Source: "));
                    ui.text_edit_singleline(&mut self.source_search);
                    let possible_source_names = self
                        .sources
                        .iter()
                        .enumerate()
                        .filter_map(|(i, source)| {
                            if let Some(source) = source {
                                if source.name.starts_with(&self.source_search) {
                                    Some((i, source.name.clone()))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<(usize, String)>>();
                    let buttons = possible_source_names
                        .iter()
                        .map(|(i, s)| (*i, ui.button(rich_text(s)).clicked()))
                        .collect::<Vec<(usize, bool)>>();
                    for (i, clicked) in buttons.iter() {
                        if *clicked {
                            self.selected_source = Some(*i);
                            break;
                        }
                    }
                });
                if let Some(source_idx) = self.selected_source {
                    ui.heading(rich_text(&self.sources[source_idx].as_ref().unwrap().name));
                    let mut text = None;
                    let mut series_2d = None;
                    let mut image = None;
                    ui.horizontal(|ui| {
                        text = Some(ui.button(rich_text("text")));
                        series_2d = Some(ui.button(rich_text("series_2d")));
                        image = Some(ui.button(rich_text("image")));
                    });
                    if text.unwrap().clicked() {
                        self.mode = Some(PlotMode::Text());
                    } else if series_2d.unwrap().clicked() {
                        self.mode = Some(PlotMode::Series2d());
                    } else if image.unwrap().clicked() {
                        self.mode = Some(PlotMode::Image());
                    }
                    match &self.mode {
                        Some(m) => match m {
                            PlotMode::Text() => {
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        for t in &self.sources[source_idx].as_ref().unwrap().texts {
                                            let text = match t {
                                                (Some(t), text) => rich_text(
                                                    &("[".to_string() + &t.1 + "]: " + &text),
                                                )
                                                .color(Color32::from_rgb(t.0 .0, t.0 .1, t.0 .2)),
                                                (None, text) => rich_text(text),
                                            };
                                            ui.label(text);
                                        }
                                    });
                            }
                            PlotMode::Series2d() => {
                                ui.horizontal(|ui| {
                                    ui.label(rich_text("History:"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.series_2d_history)
                                            .speed(1.0),
                                    );
                                    ui.label(rich_text("Plot ratios:"));
                                    ui.add(egui::Slider::new(&mut self.plot_ratios, 0.1..=10.));
                                });
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for (plot_name, plot) in
                                        &self.sources[source_idx].as_ref().unwrap().series_plots_2d
                                    {
                                        ui.label(rich_text(plot_name));
                                        plot::Plot::new(plot_name)
                                            .view_aspect(self.plot_ratios)
                                            .legend(plot::Legend::default())
                                            .show(ui, |plot_ui| {
                                                for (name, (color, vec)) in plot {
                                                    let plot_vec = {
                                                        if self.series_2d_history <= 0.0 {
                                                            self.series_2d_history = 0.0;
                                                            vec.iter().cloned().collect()
                                                        } else if let Some(start) =
                                                            vec.iter().position(|&v| {
                                                                v.x > vec.back().unwrap().x
                                                                    - self.series_2d_history
                                                            })
                                                        {
                                                            vec.range(start..)
                                                                .cloned()
                                                                .collect::<Vec<_>>()
                                                        } else {
                                                            vec![]
                                                        }
                                                    };
                                                    let line = plot::Line::new(
                                                        plot::PlotPoints::Owned(plot_vec),
                                                    )
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
                                        &self.sources[source_idx].as_ref().unwrap().image_plots
                                    {
                                        ui.label(rich_text(image_name));
                                        plot_image.show(ui);
                                    }
                                });
                            }
                        },
                        None => (),
                    }
                }
            }
        });
    }
}