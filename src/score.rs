mod score {
    use ndarray::Array1;
    use ntpnet_lib::TransitionMaker;
    #[derive(ntpnet_macro::TransitionInputTokens)]
    pub struct A {
        pub a: Vec<String>,
    }
    #[derive(ntpnet_macro::TransitionInputTokens)]
    pub struct B {
        pub b: ndarray::Array1<i32>,
    }
    #[derive(ntpnet_macro::TransitionInputTokens)]
    pub struct AB {
        pub a: Vec<String>,
        pub b: ndarray::Array1<i32>,
    }
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    pub struct C {
        pub c: f64,
    }
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    pub struct D {
        pub d: f64,
    }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(f: Input(A, B) -> Output(C, D))]
    #[ntpnet_transition(f2: Input2(AB) -> Output2(D))]
    pub struct Score {}
    impl Score {
        pub fn maker() -> TransitionMaker {
            Box::new(move || Box::new(Self {}))
        }
        fn f(&mut self, f: Input) -> Output {
            match f {
                Input::A(A { a }) => {}
                Input::B(B { b }) => {}
            };
            Output::C(C { c: 0.0 })
        }
        fn f2(&mut self, f: Input2) -> Output2 {
            let AB { a, b } = match f {
                Input2::AB(AB) => AB,
            };
            Output2::D(D { d: 0.0 })
        }
    }
}

use ntpnet_lib::{net::Net, reactor::Reactor, Token};
use plotmux::plotmux::PlotMux;

use ndarray::array;
use std::thread;

fn main() {
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        //.set_start_tokens("A", vec![Box::new(vec!["xyz".to_string()])])
        //.set_start_tokens("B", vec![Box::new(array![0_i32, 1_i32, 0_i32])])
        .place_to_transition("X", "a", "score")
        .place_to_transition("Y", "b", "score")
        .transition_to_place("score", "c", "Z")
        .transition_to_place("score", "d", "Q")
        .add_transition("score", score::Score::maker());
    plotmux.make_ready(&n.png());
    thread::spawn(move || plotmux.spin());
    Reactor::make(n).run();
}
