use crate::plotmux::{color, Color, PlotableData};
use eframe::egui::widgets::plot::Value;
use std::collections::HashMap;

pub struct PlotSource {
    pub name: String,
    pub color: Color,
    pub text: Vec<String>,
    pub series_2d: HashMap<String, (Color, Vec<Value>)>,
}
impl PlotSource {
    pub fn new_data(&mut self, d: PlotableData) {
        match d {
            PlotableData::String(s) => self.text.push(s.s),
            PlotableData::Series2d(series_2d) => match self.series_2d.get_mut(&series_2d.series) {
                Some(points) => {
                    points.1.push(Value::new(series_2d.x, series_2d.y));
                }
                None => {
                    let c = color(&series_2d.series);
                    self.series_2d.insert(
                        series_2d.series,
                        (c, vec![Value::new(series_2d.x, series_2d.y)]),
                    );
                }
            },
        }
    }
}
