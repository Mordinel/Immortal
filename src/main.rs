mod immortal;
use immortal::Immortal;

fn main() {
    let socket_str = "127.0.0.1:7777";

    let immortal = match Immortal::new(socket_str) {
        Err(e) => panic!("{:?}", e),
        Ok(i) => i,
    };
    immortal.listen();
}
