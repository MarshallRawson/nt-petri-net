use soundio;
use crossbeam_channel;
use ouroboros::self_referencing;

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

#[self_referencing]
struct SoundIOStuff<'a> {
    ctx: soundio::Context<'a>,
    #[borrows(ctx)]
    #[covariant]
    input_dev: soundio::Device<'this>,
    #[borrows(input_dev)]
    #[covariant]
    in_stream: soundio::InStream<'this>,
}

struct SoundIOCallback {
    out_buf: crossbeam_channel::Sender<Vec<i16>>,
}
impl SoundIOCallback {
    #[allow(dead_code)] // rustc cant deduce that this actually is used
    fn f(&mut self, stream: &mut soundio::InStreamReader) {
        let mut frames_left = stream.frame_count_max();
        let mut ret = vec![];
        loop {
            if let Err(e) = stream.begin_read(frames_left) {
                println!("Error reading from stream: {}", e);
                return;
            }
            for f in 0..stream.frame_count() {
                ret.push(stream.sample::<i16>(0, f))
            }
            frames_left -= stream.frame_count();
            if frames_left <= 0 {
                break;
            }
            stream.end_read();
        }
        self.out_buf.send(ret).unwrap();
    }
}

#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(f: Input(Enable) -> Output(Samples))]
pub struct SoundReader<'a> {
    p: PlotSink,
    soundio_stuff: SoundIOStuff<'a>,
    rx: crossbeam_channel::Receiver<Vec<i16>>,
    first: bool,
    count: usize,
}
impl SoundReader<'_> {
    pub fn maker(plotsink: PlotSink) -> ntpnet_lib::TransitionMaker {
        Box::new(move || {
            let mut ctx = soundio::Context::new();
            ctx.set_app_name("ntpetrinets");
            ctx.connect().unwrap();
            ctx.flush_events();
            let sample_rate = 44100;
            let soundio_format = soundio::Format::S16LE;
            let default_layout = soundio::ChannelLayout::get_default(1 as _);
            let (sender, rx) = crossbeam_channel::unbounded();
            let mut soundio_cb = SoundIOCallback { out_buf: sender };
            Box::new(
                SoundReader {
                    p: plotsink,
                    soundio_stuff: SoundIOStuffBuilder {
                        ctx: ctx,
                        input_dev_builder: |ctx: &soundio::Context| {
                            ctx.default_input_device().expect("could not open device")
                        },
                        in_stream_builder: |input_dev: &soundio::Device| {
                            input_dev.open_instream(
                                sample_rate as _,
                                soundio_format,
                                default_layout,
                                0.1,
                                move |x| soundio_cb.f(x),
                                None::<fn()>,
                                None::<fn(soundio::Error)>,
                            ).unwrap()
                        },
                    }.build(),
                    rx: rx,
                    first: false,
                    count: 0,
                }
            )
        })
    }
    fn f(&mut self, _i: Input) -> Output {
        if self.first {
            self.first = false;
            self.soundio_stuff.with_mut(|fields| {
                fields.in_stream.start().unwrap();
            });
        }
        let samples = self.rx.iter().flatten().collect::<Vec<_>>();
        for (i, x) in samples.iter().enumerate(){
            self.p.plot_series_2d("", "", (self.count + i) as _, *x as _);
        }
        Output::Samples(Samples { samples: () })
    }
}



