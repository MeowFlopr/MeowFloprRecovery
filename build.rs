fn main() {
    println!("cargo:rerun-if-changed=c_src/display_drm.c");
    println!("cargo:rerun-if-changed=c_src/display_drm.h");
    println!("cargo:rerun-if-changed=c_src/libdrm/xf86drm.c");
    println!("cargo:rerun-if-changed=c_src/libdrm/xf86drmMode.c");

    cc::Build::new()
        .file("c_src/display_drm.c")
        .file("c_src/libdrm/xf86drm.c")
        .file("c_src/libdrm/xf86drmMode.c")
        .file("c_src/libdrm/xf86drmHash.c")
        .file("c_src/libdrm/xf86drmRandom.c")
        .include("c_src")
        .include("c_src/libdrm")
        .include("c_src/libdrm/drm")
        .define("_GNU_SOURCE", None)
        .flag("-U_FORTIFY_SOURCE")
        .flag("-D_FORTIFY_SOURCE=0")
        .flag("-include")
        .flag("sys/sysmacros.h")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-sign-compare")
        .flag("-Wno-enum-conversion")
        .flag("-Wno-format-truncation")
        .compile("mrr_drm");
}
