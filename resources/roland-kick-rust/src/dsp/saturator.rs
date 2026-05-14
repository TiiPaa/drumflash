pub fn soft_clip(x: f32, drive: f32) -> f32 {
    let drive = drive.max(0.0);
    let driven = x * (1.0 + drive * 4.0);
    driven.tanh()
}
