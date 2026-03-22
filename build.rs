fn main() {
    cc::Build::new().file("src/service.c").compile("service_c")
}