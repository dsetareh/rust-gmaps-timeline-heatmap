use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::Read;
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
struct Location {
    latitudeE7: Option<f64>,
    longitudeE7: Option<f64>,
    accuracy: Option<f64>,
}

fn read_json_file(file_path: &str) -> Vec<Location> {
    let mut file = File::open(file_path).expect("File not found");
    let mut data = String::new();
    file.read_to_string(&mut data)
        .expect("Error reading the file");

    let json_data: Value = serde_json::from_str(&data).expect("Error parsing JSON");
    let locations = json_data["locations"]
        .as_array()
        .expect("locations should be an array");
    let mut locs = Vec::new();

    for loc in locations {
        let latitude = loc["latitudeE7"].as_f64();
        let longitude = loc["longitudeE7"].as_f64();
        let accuracy = loc["accuracy"].as_f64();

        if accuracy.is_none() {
            continue;
        }

        let location = Location {
            latitudeE7: latitude,
            longitudeE7: longitude,
            accuracy: accuracy,
        };
        locs.push(location);
    }

    locs
}
fn convert_to_heatmap_data(locations: &Vec<Location>) -> Vec<(f64, f64, f64)> {
    let valid_locations: Vec<(f64, f64, f64)> = locations
        .par_iter()
        .filter_map(|loc| {
            if let (Some(latitude), Some(longitude), Some(accuracy)) =
                (loc.latitudeE7, loc.longitudeE7, loc.accuracy)
            {
                Some((latitude / 1e7, longitude / 1e7, accuracy * accuracy))
            } else {
                Some((
                    loc.latitudeE7.unwrap_or(0.0) / 1e7,
                    loc.longitudeE7.unwrap_or(0.0) / 1e7,
                    -100.0,
                ))
            }
        })
        .collect();

    valid_locations
}
fn generate_heatmap_html(heatmap_data: &Vec<(f64, f64, f64)>, output_path: &str) {
    let coordinates_json =
        serde_json::to_string(&heatmap_data).expect("Failed to serialize coordinates");

    let html = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Heatmap</title>
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <link rel="stylesheet" href="https://unpkg.com/leaflet@1.7.1/dist/leaflet.css" />
            <script src="https://unpkg.com/leaflet@1.7.1/dist/leaflet.js"></script>
            <script src="https://cdnjs.cloudflare.com/ajax/libs/leaflet.heat/0.2.0/leaflet-heat.js"></script>
            <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0-alpha1/dist/css/bootstrap.min.css" rel="stylesheet" crossorigin="anonymous">
            <script src="https://unpkg.com/@popperjs/core@2" crossorigin="anonymous"></script>
        </head>
        <body>
        <div class="container">
        <div class="row">
            <div class="col">
            <h3>Statistics</h3>
            <div id="numdatapt">Number of data points: </div>
            </div>
            <div class="col">
            <h3>Filters</h3>
            <form>
                <label id="acclabl" for="filter_accuracy">Accuracy:</label>
                <input type="range" style="width: 100%;"  id="filter_accuracy" name="filter_accuracy" min="0" max="4000000" step="100000" value="4000000">
                <br>
                <input type="checkbox" id="include_no_accuracy" name="include_no_accuracy">
                <label for="include_no_accuracy">Include entries with no accuracy value</label>
            </form>
            </div>
        </div>
        </div>
        <div id="map" style="width: 100%; height: 100vh;"></div>
        <script>
            let coordinates = {coordinates_json};
            let min_accuracy = 0;
            let include_no_accuracy = false;
                const map = L.map('map').setView([{:.8}, {:.8}], 13);
                L.tileLayer("https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png", {{
                    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
                }}).addTo(map);
            
                let heat = L.heatLayer(coordinates).addTo(map);
        
                document.getElementById('filter_accuracy').addEventListener('input', function(event) {{
                    min_accuracy = parseFloat(event.target.value);
                    update_heatmap();
                }});
        
                document.getElementById('include_no_accuracy').addEventListener('change', function(event) {{
                    include_no_accuracy = event.target.checked;
                    update_heatmap();
                }});

                function update_heatmap() {{
                    const filtered_coordinates = coordinates.filter(function(coord) {{
                        if (include_no_accuracy) {{
                            return coord[2] < min_accuracy || coord[2] < 0;
                        }} else {{
                            return coord[2] < min_accuracy && coord[2] > 0;
                        }}
                        document.getElementById('acclabl').innerHTML = "Accuracy: " + min_accuracy;
                        document.getElementById('numdatapt').innerHTML = "Number of data points: " + filtered_coordinates.length;
                    }});
        
                    map.removeLayer(heat);
                    heat = L.heatLayer(filtered_coordinates).addTo(map);
                }}
            </script>
        </body>
        </html>
            "#,
        heatmap_data[0].0,
        heatmap_data[0].1,
        coordinates_json = coordinates_json
    );

    let mut file = File::create(output_path).expect("Failed to create output file");
    file.write_all(html.as_bytes())
        .expect("Failed to write to output file");
}

fn main() {
    let file_name = "Records.json";
    let output_path = "heatmap.html";

    let locations = read_json_file(file_name);
    let heatmap_data = convert_to_heatmap_data(&locations);
    generate_heatmap_html(&heatmap_data, output_path);

    println!("Heatmap generated in {}", output_path);
}
