use crate::plotmux::PlotableData;
use crate::plotsource::PlotSource;
use bincode;
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe;
use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::widgets::plot;
use eframe::egui::widget_text::RichText;
use egui_extras::image::RetainedImage;
use image::io::Reader as ImageReader;
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
    fn make(port: u32) -> (Self, Receiver<(usize, PlotableData)>) {
        loop {
            match TcpStream::connect(format!("localhost:{}", port)) {
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
        thread::spawn(move || loop {
            let mut len_buf =
                vec![0; bincode::serialized_size::<usize>(&0_usize).unwrap() as usize];
            self.stream.read_exact(&mut len_buf).unwrap();
            let mut data_buf = vec![0; bincode::deserialize(&len_buf).unwrap()];
            self.stream.read_exact(&mut data_buf).unwrap();
            self.sender
                .send(bincode::deserialize(&data_buf).unwrap())
                .unwrap();
            ctx.request_repaint();
        });
    }
}

enum PlotMode {
    Text(),
    Series2d(),
    Image(),
}
pub struct PlotMuxUi {
    sources: Vec<PlotSource>,
    port: u32,
    receiver: Option<Receiver<(usize, PlotableData)>>,
    source_search: String,
    graph_image: Option<RetainedImage>,
    show_graph: bool,
    selected_source: Option<usize>,
    mode: Option<PlotMode>,
    series_2d_history: f64,
    font_size: f32,
}
impl PlotMuxUi {
    pub fn make(graph_png_path: Option<&String>, port: u32, source_names: Vec<String>) -> Self {
        let mut sources = Vec::<_>::new();
        for name in &source_names {
            sources.push(PlotSource::make(name.clone()));
        }
        let graph_image = if let Some(graph_png_path) = graph_png_path {
            let graph_image = ImageReader::open(graph_png_path).unwrap();
            let graph_image = graph_image.decode().unwrap();
            let graph_image = graph_image.as_rgba8().unwrap();
            Some(RetainedImage::from_color_image(
                "graph image",
                egui::ColorImage::from_rgba_unmultiplied(
                    [graph_image.width() as _, graph_image.height() as _],
                    graph_image.as_raw(),
                ),
            ))
        } else {
            None
        };
        PlotMuxUi {
            sources: sources,
            port: port,
            receiver: None,
            source_search: "".into(),
            graph_image: graph_image,
            show_graph: false,
            selected_source: None,
            mode: None,
            series_2d_history: 0.0,
            font_size: 15.0,
        }
    }
    pub fn spin(mut self) {
        let native_options = eframe::NativeOptions::default();
        eframe::run_native(
            "PlotMux",
            native_options,
            Box::new(|cc| {
                let (tcp_handler, rx) = TcpHandler::make(self.port);
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
            self.sources[idx].new_data(new_data);
        }
        let rich_text =  {
            let font = self.font_size;
            move |string: &str| { RichText::new(string).size(font) }
        };
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(rich_text("Font size:"));
                ui.add(egui::DragValue::new(&mut self.font_size).clamp_range(5..=100));
                ui.checkbox(&mut self.show_graph, rich_text("Graph"));
            });
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
                            if source.name.starts_with(&self.source_search) {
                                Some((i, source.name.clone()))
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
                    ui.heading(rich_text(&self.sources[source_idx].name));
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
                                        for t in &self.sources[source_idx].texts {
                                            let text = match t {
                                                (Some(t), text) => rich_text(
                                                    &("[".to_string() + &t.1 + "]: " + &text)
                                                ).color(Color32::from_rgb(t.0.0, t.0.1, t.0.2)),
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
                                });
                                egui::ScrollArea::vertical()
                                    .show(ui, |ui| {
                                        for (plot_name, plot) in
                                            &self.sources[source_idx].series_plots_2d {
                                            ui.label(rich_text(plot_name));
                                            plot::Plot::new(plot_name)
                                                .view_aspect(4.0)
                                                .legend(plot::Legend::default())
                                                .show(ui, |plot_ui| {
                                                    for (name, (color, vec)) in plot
                                                    {
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
                                egui::ScrollArea::both()
                                    .show(ui, |ui| {
                                        for (image_name, plot_image) in &self.sources[source_idx].image_plots {
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
