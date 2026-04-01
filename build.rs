use std::env;
use std::path::PathBuf;

/*

/usr/bin/c++
-I/usr/local/cuda/include
CMakeFiles/vpi_sample_01_convolve_2d.dir/main.cpp.o -o vpi_sample_01_convolve_2d
-L/usr/local/cuda/lib64
-Wl,-rpath,/usr/local/cuda/lib64:/opt/nvidia/vpi3/lib/x86_64-linux-gnu:/usr/local/lib /opt/nvidia/vpi3/lib/x86_64-linux-gnu/libnvvpi.so.3.0.10 /usr/local/lib/libopencv_imgcodecs.so.4.13.0 /usr/local/lib/libopencv_imgproc.so.4.13.0 /usr/local/lib/libopencv_core.so.4.13.0 /usr/local/lib/libopencv_cudev.so.4.13.0

*/

fn main() {
    // .so location and linker flag
    println!("cargo:rustc-link-search=native=/opt/nvidia/vpi3/lib/x86_64-linux-gnu");
    println!("cargo:rustc-link-lib=dylib=nvvpi");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-I/opt/nvidia/vpi3/include")
        .clang_arg("-I/usr/local/cuda/include")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
