//use std::path::Path;
use std::io::BufReader;
use std::fs::File;
use image::RgbImage;
use mp4;

use ntpnet_lib::TransitionMaker;
use ntpnet_macro::{Transition, TransitionInputTokens, TransitionOutputTokens};
use plotmux::plotsink::PlotSink;

#[derive(TransitionInputTokens)]
struct Enable {
    _e: (),
}

#[derive(TransitionOutputTokens)]
struct Image {
    image: RgbImage,
}

#[derive(Transition)]
#[ntpnet_transition(read_frame: Input(Enable) -> Output(Image))]
pub struct MP4Reader {
    p: PlotSink,
    mp4: mp4::Mp4Reader<BufReader<File>>,
    sample: u32,
}

impl MP4Reader {
    pub fn maker(mut p: PlotSink, path: String) -> TransitionMaker {
        Box::new(move || {
            let f = File::open(&path).expect(&format!("Cannot open file {:?}", path));
            let size = f.metadata().unwrap().len();
            let reader = BufReader::new(f);
            let mp4 = mp4::Mp4Reader::read_header(reader, size).unwrap();

            p.println(&format!("{:#?}", mp4.tracks().iter().map(|(i, t)| (i, &t.trak))));

            Box::new(Self {
                p: p,
                mp4: mp4,
                sample: 0,
            })
        })
    }
    fn read_frame(&mut self, _i: Input) -> Output {
        Output::Image(Image { image: RgbImage::new(0, 0)})
    }
}
