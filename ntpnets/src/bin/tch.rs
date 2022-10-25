use anyhow::Result;
use tch::{nn, nn::ModuleT, nn::OptimizerConfig, Device, Tensor};


#[derive(Debug)]
struct Net {
    conv1: nn::Conv2D,
    conv2: nn::Conv2D,
    fc1: nn::Linear,
    fc2: nn::Linear,
}
impl Net {
    fn new(vs: &nn::Path) -> impl ModuleT {
        let conv1 = nn::conv2d(vs, 1, 28, 5, Default::default());
        let conv2 = nn::conv2d(vs, 28, 64, 5, Default::default());
        let fc1 = nn::linear(vs, 1024, 1024, Default::default());
        let fc2 = nn::linear(vs, 1024, 10, Default::default());
        Net { conv1, conv2, fc1, fc2 }
    }
}
impl nn::ModuleT for Net {
    fn forward_t(&self, xs: &Tensor, train: bool) -> Tensor {
        xs.view([-1, 1, 28, 28])
            .apply(&self.conv1)
            .max_pool2d_default(2)
            .apply(&self.conv2)
            .max_pool2d_default(2)
            .view([-1, 1024])
            .apply(&self.fc1)
            .relu()
            .dropout(0.5, train)
            .apply(&self.fc2)
    }
}

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 30)]
    fps: u32,
    #[arg(short, long)]
    remote_plotmux: Option<String>,
}

pub fn main() -> Result<()> {
    let args = Args::parse();
    use plotmux::plotmux::{PlotMux, ClientMode};
    let mut plotmux = PlotMux::make();
    let mut plotsink = plotmux.add_plot_sink("training");
    let plotmux_mode = if let Some(addr) = args.remote_plotmux {
        ClientMode::Remote(addr)
    } else {
        ClientMode::Local()
    };
    plotmux.make_ready(None, plotmux_mode);
    let data_path = std::env::current_exe()
        .expect("Getting current exe")
        .as_path()
        .parent().unwrap()
        .parent().unwrap()
        .join(std::path::Path::new("data"));
    let m = tch::vision::mnist::load_dir(&data_path)?;
    let vs = nn::VarStore::new(Device::Cpu);
    let net = Net::new(&vs.root());
    let mut opt = nn::Adam::default().build(&vs, 1e-3)?;
    for epoch in 1..100 {
        for (bimages, blabels) in m.train_iter(256).shuffle().to_device(vs.device()) {
            let loss = net.forward_t(&bimages, true).cross_entropy_for_logits(&blabels);
            opt.backward_step(&loss);
        }
        let test_accuracy = net.batch_accuracy_for_logits(&m.test_images, &m.test_labels, vs.device(), 1024);
        plotsink.plot_series_2d("", "test acc", epoch as f64, test_accuracy);
        plotsink.println(&format!(
            "epoch: {:4} test acc: {:5.2}%",
            epoch,
            100. * test_accuracy,
        ));
    }
    Ok(())
}
