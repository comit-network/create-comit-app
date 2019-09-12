use create_comit_app::cnd::Cnd;

fn main() {
    Cnd::start();
    std::thread::park();
}
