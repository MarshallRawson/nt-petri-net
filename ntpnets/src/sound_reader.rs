use libpulse_simple_binding::Simple;
use libpulse_sys::stream::pa_stream_direction_t as Direction;
use libpulse_binding::sample::{Spec, Format};
use std::time::Instant;

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
    block: usize,
    count: usize,
    last_time: Option<Instant>,
    start_time: Instant,
}
impl SoundReader {
    pub fn maker(plotsink: PlotSink) -> ntpnet_lib::TransitionMaker {
        Box::new(move || {
            let spec = Spec {
                format: Format::S16NE,
                channels: 1,
                rate: 44100,
            };
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
                    block: f64::round(0.1 * spec.rate as f64) as usize, // 0.1 sec * (samples / sec)
                    count: 0,
                    last_time: None,
                    start_time: Instant::now(),
                }
            )
        })
    }

    fn f(&mut self, _i: Input) -> Output {
        let mut samples = vec![0; self.block];
        match self.simp.read(samples.as_mut_slice()) {
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
        self.last_time = Some(now);
        self.count += 1;
        Output::Samples(Samples { samples: () })
    }
}

