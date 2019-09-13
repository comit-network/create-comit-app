use create_comit_app::cnd::Cnd;

fn main() {
    Cnd::start(8000);
    std::thread::park();
}
