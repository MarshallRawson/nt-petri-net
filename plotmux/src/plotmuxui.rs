use crate::plotsource::{PlotSource};
use crate::plotmux::{PlotableData, color};
use std::collections::HashMap;
use std::net::{TcpStream};
use bincode;
use std::io::{Read, Write};
use std::thread::sleep;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Sender, Receiver, unbounded};

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
                },
                Ok(stream) => {
                    let (tx, rx) = unbounded();
                    return (Self { stream: stream, sender: tx }, rx);
                },
            }
        }
    }
    fn spin(mut self, ctx: egui::Context) {
        thread::spawn(move || {
            loop {
                let mut len_buf = vec![0; bincode::serialized_size::<usize>(&0_usize).unwrap() as usize];
                self.stream.read_exact(&mut len_buf).unwrap();
                let mut data_buf = vec![0; bincode::deserialize(&len_buf).unwrap()];
                self.stream.read_exact(&mut data_buf).unwrap();
                self.sender.send(bincode::deserialize(&data_buf).unwrap()).unwrap();
                ctx.request_repaint();
            }
        });
    }
}

enum PlotMode {
    Text(),
    Series2d(),
}
pub struct PlotMuxUi {
    sources: Vec<PlotSource>,
    port: u32,
    receiver: Option<Receiver<(usize, PlotableData)>>,
    source_search: String,
    selected_source: Option<usize>,
    mode: Option<PlotMode>,
    series_2d_history: f64,
}
impl PlotMuxUi {
    pub fn make(port: u32, source_names: Vec<String>) -> Self {
        let mut sources = Vec::<_>::new();
        for name in &source_names {
            sources.push(PlotSource {
                name: name.clone(),
                color: color(&name),
                text: vec![],
                series_2d: HashMap::new(),
            });
        }
        PlotMuxUi {
            sources: sources,
            port: port,
            receiver: None,
            source_search: "".into(),
            selected_source: None,
            mode: None,
            series_2d_history: 0.0,
        }
    }
    pub fn spin(mut self) {
        loop {
            let (tcp_handler, rx) = TcpHandler::make(self.port);
            self.receiver = Some(rx);
            let native_options = eframe::NativeOptions::default();
            eframe::run_native("PlotMux", native_options, Box::new(|cc| {
                tcp_handler.spin(cc.egui_ctx.clone());
                Box::new(self)
            }));
        }
    }
}

use eframe;
use eframe::egui;
use eframe::egui::widgets::plot;
impl eframe::App for PlotMuxUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok((idx, new_data)) = self.receiver.as_ref().unwrap().try_recv() {
            self.sources[idx].new_data(new_data);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Source: ");
                ui.text_edit_singleline(&mut self.source_search);
                let possible_source_names = self.sources.iter().filter(|source| source.name.starts_with(&self.source_search))
                    .map(|source| source.name.clone()).collect::<Vec<String>>();
                let buttons = possible_source_names.iter().map(|s| ui.button(s).clicked()).collect::<Vec<bool>>();
                for (i, clicked) in buttons.iter().enumerate() {
                    if *clicked {
                        self.selected_source = Some(i);
                        break;
                    }
                }
            });
            if let Some(source_idx) = self.selected_source {
                ui.heading(&self.sources[source_idx].name);
                let mut text = None;
                let mut series_2d = None;
                ui.horizontal(|ui| {
                    text = Some(ui.button("text"));
                    series_2d = Some(ui.button("series_2d"));
                });
                if text.unwrap().clicked() {
                    self.mode = Some(PlotMode::Text());
                }
                else if series_2d.unwrap().clicked() {
                    self.mode = Some(PlotMode::Series2d());
                }
                match &self.mode {
                    Some(m) => match m {
                        PlotMode::Text() => {
                            egui::ScrollArea::vertical().stick_to_bottom().show(ui, |ui| {
                                for t in &self.sources[source_idx].text {
                                    ui.label(t);
                                }
                            });
                        },
                        PlotMode::Series2d() => {
                            ui.horizontal(|ui| {
                                ui.label("History:");
                                ui.add(egui::DragValue::new(&mut self.series_2d_history).speed(1.0));
                            });
                            plot::Plot::new("plot")
                                .view_aspect(2.0)
                                .legend(plot::Legend::default())
                                .show(ui, |plot_ui| {
                                    for (name, (color, vec)) in &self.sources[source_idx].series_2d {
                                        let plot_vec = {
                                            if self.series_2d_history <= 0.0 {
                                                self.series_2d_history = 0.0;
                                                vec.clone()
                                            }
                                            else if let Some(start) = vec.iter().position(|&v| v.x > vec.last().unwrap().x - self.series_2d_history) {
                                                vec[start..].to_vec()
                                            }
                                            else {
                                                vec![]
                                            }
                                        };
                                        let line = plot::Line::new(plot::Values::from_values(plot_vec.clone()))
                                            .name(name)
                                            .color(egui::Color32::from_rgb(color.0, color.1, color.2))
                                        ;
                                        plot_ui.line(line);
                                    }
                                })
                            ;
                        },
                    },
                    None => (),
                }
            }
       });
    }
}
