use vpi::sys;

fn main() {
    println!("Calling VPI function...");
    unsafe {
        let mut stream = std::ptr::null_mut();
        let status = sys::vpiStreamCreate(0, &mut stream);
        println!("vpiStreamCreate returned: {status}");
    }
}
