const SATP_SV39: usize = 8 << 60;
enum PageField {
    V = 1 << 0,
    R = 1 << 1,
    W = 1 << 2,
    X = 1 << 3,
    U = 1 << 4,
}