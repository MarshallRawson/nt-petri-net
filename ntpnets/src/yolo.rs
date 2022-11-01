use image::RgbImage;
use anyhow::Result;
use tch;
use tch::nn::ModuleT;

use plotmux::plotsink::PlotSink;
use ntpnet_lib::TransitionMaker;

use crate::coco_classes;
use crate::darknet;

use ouroboros::self_referencing;

#[self_referencing]
struct Model {
    v: tch::nn::VarStore,
    darknet: darknet::Darknet,
    #[borrows(v, darknet)]
    #[covariant]
    f: tch::nn::FuncT<'this>
}

#[derive(ntpnet_macro::TransitionOutputTokens)]
pub struct Out {
    out: (),
}
#[derive(ntpnet_macro::TransitionInputTokens)]
pub struct Image {
    image: RgbImage,
}

#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(f: Input(Image) -> Output(Out))]
pub struct Yolo {
    p: PlotSink,
    model: Model,
    dims: (i64, i64),
}

impl Yolo {
    pub fn maker(mut plotsink: PlotSink) -> TransitionMaker {
        let mut vs = tch::nn::VarStore::new(tch::Device::Cpu);
        let config_path = std::env::current_exe()
            .expect("Getting current exe")
            .as_path()
            .parent().unwrap()
            .parent().unwrap()
            .parent().unwrap()
            .join(std::path::Path::new("ntpnets"))
            .join(std::path::Path::new("src"))
            .join(std::path::Path::new("yolo-v3.cfg"));
        let darknet = darknet::parse_config(&config_path).unwrap();
        let weights_path = std::env::current_exe()
            .expect("Getting current exe")
            .as_path()
            .parent().unwrap()
            .parent().unwrap()
            .join(std::path::Path::new("yolo"))
            .join(std::path::Path::new("yolo-v3.ot"));
        vs.load(&weights_path).unwrap();
        let input_dims = (darknet.width().unwrap(), darknet.height().unwrap());
        plotsink.println(&format!("model input image dims: {:?}", input_dims));
        let model = ModelBuilder {
            v: vs,
            darknet: darknet,
            f_builder: move |v, darknet| darknet.build_model(&v.root()).unwrap(),
        }.build();
        Box::new(move || Box::new(Self {
            p: plotsink,
            model: model,
            dims: input_dims,
        }))
    }
    fn f(&mut self, i: Input) -> Output {
        let image = match i {
            Input::Image(Image { image }) => image,
        };
        let image = tch::vision::image::load_and_resize_from_memory(&image.as_raw(), self.dims.0, self.dims.1).unwrap();
        let image = image.unsqueeze(0).to_kind(tch::Kind::Float) / 255.;
        self.p.println(&format!("{:#?}", (*self.model.borrow_f()).forward_t(&image, false).squeeze()));
        Output::Out(Out { out: () })
    }
}
