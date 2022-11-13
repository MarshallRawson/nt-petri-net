use libpulse_simple_binding::Simple;
use libpulse_sys::stream::pa_stream_direction_t as Direction;
use libpulse_binding::sample::{Spec, Format};
use std::time::Instant;
use byteorder::{ByteOrder, LittleEndian};

use ntpnet_macro;
use ntpnet_lib;
use plotmux::plotsink::PlotSink;

#[derive(ntpnet_macro::TransitionInputTokens)]
struct Enable {
    _e: (),
}

#[derive(ntpnet_macro::TransitionOutputTokens)]
struct Samples {
    samples: (),//Vec::<i16>,
}

#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(f: Input(Enable) -> Output(Samples))]
pub struct SoundReader {
    p: PlotSink,
    simp: Simple,
    count: usize,
    last_time: Option<Instant>,
    start_time: Instant,
    data: Vec<u8>,
    sample_block: usize,
    samples: Vec<(f64, f64)>,
}
impl SoundReader {
    pub fn maker(plotsink: PlotSink) -> ntpnet_lib::TransitionMaker {
        Box::new(move || {
            let spec = Spec {
                format: Format::S16NE,
                channels: 1,
                rate: 44100,
            };
            let bytes_per_sample = 2;
            // 0.1 sec * (samples / sec) * (bytes / sample)
            let data_block = f64::round(0.1 * spec.rate as f64) as usize * bytes_per_sample;
            // 0.1 sec * (samples / sec)
            let sample_block = f64::round(0.1 * spec.rate as f64) as usize;
            Box::new(
                SoundReader {
                    p: plotsink,
                    simp: Simple::new(
                        None,                // Use the default server
                        "FooApp",            // Our application’s name
                        Direction::Record,   // We want a playback stream
                        None,                // Use the default device
                        "Music",             // Description of our stream
                        &spec,               // Our sample format
                        None,                // Use default channel map
                        None                 // Use default buffering attributes
                    ).unwrap(),
                    count: 0,
                    last_time: None,
                    start_time: Instant::now(),
                    data: vec![0; data_block],
                    sample_block: sample_block,
                    samples: vec![(0., 0.); sample_block],
                }
            )
        })
    }
    fn f(&mut self, _i: Input) -> Output {
        match self.simp.read(self.data.as_mut_slice()) {
            Err(e) => println!("{}", e.to_string().unwrap()),
            Ok(_) => {},
        }
        let now = Instant::now();
        if let Some(t) = self.last_time {
            self.p.plot_series_2d(
                "",
                "1 / frame period",
                (now - self.start_time).as_secs_f64(),
                1. / (now - t).as_secs_f64()
            );
        }
        for i in 0..self.samples.len() {
            self.samples[i] = (
                (self.count*self.sample_block + i) as f64, LittleEndian::read_i16(&self.data[i*2..i*2+2]) as f64
            );
        }
        self.p.plot_series_2d_vec(
            "audio",
            "audio",
            self.samples.clone(),
        );
        self.last_time = Some(now);
        self.count += 1;
        Output::Samples(Samples { samples: () })
    }
}

