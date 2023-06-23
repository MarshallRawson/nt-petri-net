use crate::plotmuxui::PlotMode;
use eframe::egui;
use serde::{Serialize, Deserialize};

pub enum Panel {
    Horizontal(egui::SidePanel),
    Vertical(egui::TopBottomPanel),
}

#[derive(Serialize, Deserialize)]
pub struct PlotPanel {
    pub name: String,
    // left, right, up, down
    children: [(String, Option<Box<PlotPanel>>); 4],
    series_2d_history: f64,
    plot_height: f32,
    show_graph: bool,
    source_search: String,
    source: Option<(PlotMode, usize)>,
}

#[derive(Serialize, Deserialize)]
enum Child {
    Left = 0,
    Down,
    Up,
    Right,
}

impl PlotPanel {
    pub fn new(name: String) -> Self {
        Self {
            name: name,
            children: [("<".into(), None), ("\\/".into(), None), ("/\\".into(), None), (">".into(), None)],
            series_2d_history: 0.0,
            plot_height: 200.0,
            show_graph: false,
            source_search: "".into(),
            source: None,
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        panel: Panel,
        rich_text: &dyn Fn(&str) -> egui::RichText,
        source_search: &dyn Fn(&mut egui::Ui, &mut bool, &mut String) -> Option<(PlotMode, usize)>,
        plot_graph: &dyn Fn(&mut egui::Ui),
        plot_source: &dyn Fn(&mut egui::Ui, usize, &PlotMode, &mut f64, &mut f32) -> bool,
    ) {
        let f = |ui: &mut egui::Ui| {
            tile(
                ui,
                &|n: String| Panel::Horizontal(egui::panel::SidePanel::left(n).resizable(true)),
                self.name.clone(),
                Child::Left,
                &mut self.children,
                rich_text,
                source_search,
                plot_graph,
                plot_source,
            );
            tile(
                ui,
                &|n: String| Panel::Horizontal(egui::panel::SidePanel::right(n).resizable(true)),
                self.name.clone(),
                Child::Right,
                &mut self.children,
                rich_text,
                source_search,
                plot_graph,
                plot_source,
            );
            tile(
                ui,
                &|n: String| Panel::Vertical(egui::panel::TopBottomPanel::top(n).resizable(true)),
                self.name.clone(),
                Child::Up,
                &mut self.children,
                rich_text,
                source_search,
                plot_graph,
                plot_source,
            );
            ui.horizontal(|ui| {
                ui.label(rich_text(&self.name));
                for (name, panel) in self.children.iter_mut() {
                    let mut button = egui::Button::new(rich_text(name));
                    if panel.is_some() {
                        button = button.fill(egui::Color32::RED);
                    }
                    if ui.add(button).clicked() {
                        if panel.is_none() {
                            *panel = Some(Box::new(Self::new(self.name.clone() + name)));
                        } else {
                            *panel = None;
                        }
                    }
                }
            });
            tile(
                ui,
                &|n: String| {
                    Panel::Vertical(egui::panel::TopBottomPanel::bottom(n).resizable(true))
                },
                self.name.clone(),
                Child::Down,
                &mut self.children,
                rich_text,
                source_search,
                plot_graph,
                plot_source,
            );
            if self.source.is_none() {
                self.source = source_search(ui, &mut self.show_graph, &mut self.source_search);
            }
            if self.show_graph {
                plot_graph(ui);
            } else {
                let exit_source = if let Some((mode, source_idx)) = &self.source {
                    !plot_source(
                        ui,
                        *source_idx,
                        mode,
                        &mut self.series_2d_history,
                        &mut self.plot_height,
                    )
                } else {
                    false
                };
                if exit_source {
                    self.source = None;
                }
            }
        };
        match panel {
            Panel::Horizontal(panel) => {
                panel.show_inside(ui, f);
            }
            Panel::Vertical(panel) => {
                panel.show_inside(ui, f);
            }
        }
    }
}

fn tile(
    ui: &mut egui::Ui,
    s: &dyn Fn(String) -> Panel,
    name: String,
    idx: Child,
    children: &mut [(String, Option<Box<PlotPanel>>); 4],
    rich_text: &dyn Fn(&str) -> egui::RichText,
    source_search: &dyn Fn(&mut egui::Ui, &mut bool, &mut String) -> Option<(PlotMode, usize)>,
    plot_graph: &dyn Fn(&mut egui::Ui),
    plot_source: &dyn Fn(&mut egui::Ui, usize, &PlotMode, &mut f64, &mut f32) -> bool,
) {
    let (n, c) = &mut children[idx as usize];
    if c.is_some() {
        c.as_mut().unwrap().show(
            ui,
            s(name + n),
            rich_text,
            source_search,
            plot_graph,
            plot_source,
        );
    }
}
