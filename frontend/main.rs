use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    
    mount_to_body(|| {
        view! {
            <div>
                <h1>"BSL Gradual Type System"</h1>
                <p>"Frontend will be implemented here"</p>
            </div>
        }
    })
}
