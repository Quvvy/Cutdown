/// How the WebView2 `<video>` element should get pixels for a source.
///
/// Native WebView2 playback is reliable for 8-bit H.264 in MP4/MOV and VP8/VP9 in WebM.
/// HEVC/AV1/10-bit sources often "load" without a media error and then show a black frame —
/// remuxing those into MP4 does not help because the codec is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewPlan {
    Native,
    Remux,
    Proxy,
}

pub fn plan_for(codec: &str, pixel_format: &str, container: &str) -> PreviewPlan {
    let codec = codec.trim().to_ascii_lowercase();
    let pixel_format = pixel_format.trim().to_ascii_lowercase();
    let container = container.trim().trim_start_matches('.').to_ascii_lowercase();

    if !pixel_format_webview_safe(&pixel_format) {
        return PreviewPlan::Proxy;
    }

    if is_h264(&codec) {
        if container_is_mp4_family(&container) {
            return PreviewPlan::Native;
        }
        return PreviewPlan::Remux;
    }

    if is_vp8_or_vp9(&codec) && container_is_webm(&container) {
        return PreviewPlan::Native;
    }

    PreviewPlan::Proxy
}

fn is_h264(codec: &str) -> bool {
    codec == "h264" || codec == "avc1" || codec == "avc" || codec.starts_with("avc1.")
}

fn is_vp8_or_vp9(codec: &str) -> bool {
    codec == "vp8" || codec == "vp9" || codec == "vp09"
}

fn pixel_format_webview_safe(pixel_format: &str) -> bool {
    pixel_format.is_empty()
        || pixel_format == "yuv420p"
        || pixel_format == "yuvj420p"
        || pixel_format == "nv12"
}

fn container_is_mp4_family(container: &str) -> bool {
    matches!(container, "mp4" | "m4v" | "mov" | "m4a" | "3gp")
}

fn container_is_webm(container: &str) -> bool {
    container == "webm"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h264_mp4_is_native() {
        assert_eq!(plan_for("h264", "yuv420p", "mp4"), PreviewPlan::Native);
        assert_eq!(plan_for("avc1", "", "MOV"), PreviewPlan::Native);
    }

    #[test]
    fn h264_mkv_is_remux() {
        assert_eq!(plan_for("h264", "yuv420p", "mkv"), PreviewPlan::Remux);
        assert_eq!(plan_for("h264", "yuv420p", "ts"), PreviewPlan::Remux);
    }

    #[test]
    fn hevc_is_always_proxy() {
        assert_eq!(plan_for("hevc", "yuv420p", "mp4"), PreviewPlan::Proxy);
        assert_eq!(plan_for("h265", "yuv420p", "mkv"), PreviewPlan::Proxy);
        assert_eq!(plan_for("hevc", "yuv420p10le", "mp4"), PreviewPlan::Proxy);
    }

    #[test]
    fn ten_bit_h264_is_proxy() {
        assert_eq!(plan_for("h264", "yuv420p10le", "mp4"), PreviewPlan::Proxy);
    }

    #[test]
    fn av1_and_prores_are_proxy() {
        assert_eq!(plan_for("av1", "yuv420p", "mp4"), PreviewPlan::Proxy);
        assert_eq!(plan_for("prores", "yuv422p10le", "mov"), PreviewPlan::Proxy);
    }

    #[test]
    fn vp9_webm_is_native() {
        assert_eq!(plan_for("vp9", "yuv420p", "webm"), PreviewPlan::Native);
        assert_eq!(plan_for("vp9", "yuv420p", "mkv"), PreviewPlan::Proxy);
    }
}
