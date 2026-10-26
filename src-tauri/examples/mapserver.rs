fn main() {
    let cache = std::env::temp_dir().join("s2mm-mapcache-dev");
    stalker2mm_lib::dev_run_map_server(27421, &cache);
}
