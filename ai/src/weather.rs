use std::time::Duration;

#[derive(serde::Deserialize)]
struct GeocodeResult {
    results: Option<Vec<GeocodeLocation>>,
}

#[derive(serde::Deserialize)]
struct GeocodeLocation {
    latitude: f64,
    longitude: f64,
}

#[derive(serde::Deserialize)]
struct WeatherResponse {
    current: WeatherCurrent,
}

#[derive(serde::Deserialize)]
struct WeatherCurrent {
    temperature_2m: f64,
}

pub fn get_weather(city: &str) -> Result<String, String> {
    let (lat, lon) = get_coordinates(city, "https://geocoding-api.open-meteo.com")?;
    let weather = get_temperature(lat, lon, "https://api.open-meteo.com")?;
    Ok(format!("{}°C", weather.temperature_2m))
}

fn get_coordinates(city: &str, base_url: &str) -> Result<(f64, f64), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let encoded_city = urlencoding::encode(city);
    let url = format!(
        "{}/v1/search?name={}&count=1&language=en&format=json",
        base_url, encoded_city
    );

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Geocoding API error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Geocoding API error: HTTP {}", response.status()));
    }

    let geocode_result: GeocodeResult = response
        .json()
        .map_err(|e| format!("Failed to parse geocoding response: {}", e))?;

    match geocode_result.results {
        Some(results) if !results.is_empty() => {
            let location = &results[0];
            Ok((location.latitude, location.longitude))
        }
        _ => Err(format!("Unknown location: {}", city)),
    }
}

fn get_temperature(lat: f64, lon: f64, base_url: &str) -> Result<WeatherCurrent, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = format!(
        "{}/v1/forecast?latitude={}&longitude={}&current=temperature_2m",
        base_url, lat, lon
    );

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Weather API error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Weather API error: HTTP {}", response.status()));
    }

    let weather_response: WeatherResponse = response
        .json()
        .map_err(|e| format!("Failed to parse weather response: {}", e))?;

    Ok(weather_response.current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn test_get_weather_mocked() {
        let mut geocoding_server = Server::new();
        let mut weather_server = Server::new();
        let geocoding_mock = geocoding_server
            .mock(
                "GET",
                "/v1/search?name=Zurich&count=1&language=en&format=json",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "results": [{
                    "latitude": 47.3769,
                    "longitude": 8.5417
                }]
            }"#,
            )
            .create();
        let weather_mock = weather_server
            .mock(
                "GET",
                "/v1/forecast?latitude=47.3769&longitude=8.5417&current=temperature_2m",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "current": {
                    "temperature_2m": 22.5
                }
            }"#,
            )
            .create();

        let (lat, lon) = get_coordinates("Zurich", &geocoding_server.url()).unwrap();
        assert_eq!(lat, 47.3769);
        assert_eq!(lon, 8.5417);

        let temperature = get_temperature(lat, lon, &weather_server.url()).unwrap();
        assert_eq!(temperature, 22.5);

        geocoding_mock.assert();
        weather_mock.assert();
    }

    #[test]
    fn test_get_coordinates_unknown_location() {
        let mut server = Server::new();

        // Mock empty results for unknown location
        let mock = server
            .mock(
                "GET",
                "/v1/search?name=NonExistentCityXYZ123&count=1&language=en&format=json",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "results": []
            }"#,
            )
            .create();

        let result = get_coordinates("NonExistentCityXYZ123", &server.url());
        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("Unknown location:"));

        mock.assert();
    }

    #[test]
    fn test_get_coordinates_no_results() {
        let mut server = Server::new();

        // Mock null results
        let mock = server
            .mock(
                "GET",
                "/v1/search?name=TestCity&count=1&language=en&format=json",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "results": null
            }"#,
            )
            .create();

        let result = get_coordinates("TestCity", &server.url());
        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("Unknown location:"));

        mock.assert();
    }

    #[test]
    fn test_geocoding_api_error() {
        let mut server = Server::new();

        // Mock server error
        let mock = server
            .mock(
                "GET",
                "/v1/search?name=TestCity&count=1&language=en&format=json",
            )
            .with_status(500)
            .create();

        let result = get_coordinates("TestCity", &server.url());
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Geocoding API error: HTTP 500"));

        mock.assert();
    }

    #[test]
    fn test_weather_api_error() {
        let mut server = Server::new();

        // Mock server error
        let mock = server
            .mock(
                "GET",
                "/v1/forecast?latitude=47.3769&longitude=8.5417&current=temperature_2m",
            )
            .with_status(500)
            .create();

        let result = get_temperature(47.3769, 8.5417, &server.url());
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Weather API error: HTTP 500"));

        mock.assert();
    }

    #[test]
    fn test_invalid_json_response() {
        let mut server = Server::new();

        // Mock invalid JSON response
        let mock = server
            .mock(
                "GET",
                "/v1/search?name=TestCity&count=1&language=en&format=json",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("invalid json")
            .create();

        let result = get_coordinates("TestCity", &server.url());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Failed to parse geocoding response")
        );

        mock.assert();
    }

    #[test]
    fn test_full_weather_flow_mocked() {
        let mut geocoding_server = Server::new();
        let mut weather_server = Server::new();

        // Mock geocoding API response for Tokyo
        let geocoding_mock = geocoding_server
            .mock(
                "GET",
                "/v1/search?name=Tokyo&count=1&language=en&format=json",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "results": [{
                    "latitude": 35.6762,
                    "longitude": 139.6503
                }]
            }"#,
            )
            .create();

        // Mock weather API response
        let weather_mock = weather_server
            .mock(
                "GET",
                "/v1/forecast?latitude=35.6762&longitude=139.6503&current=temperature_2m",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "current": {
                    "temperature_2m": 18.7
                }
            }"#,
            )
            .create();

        // Test the complete flow by creating a custom get_weather function for testing
        let (lat, lon) = get_coordinates("Tokyo", &geocoding_server.url()).unwrap();
        let temperature = get_temperature(lat, lon, &weather_server.url()).unwrap();
        let result = format!("{}°C", temperature);

        assert_eq!(result, "18.7°C");

        geocoding_mock.assert();
        weather_mock.assert();
    }

    // Keep the original integration test for reference, but it's now properly documented
    #[test]
    #[ignore] // Use `cargo test -- --ignored` to run this test
    fn test_get_weather_integration() {
        // This test requires internet connection and may be flaky
        // Run with: cargo test test_get_weather_integration -- --ignored
        let result = get_weather("Zurich");
        match result {
            Ok(temp) => {
                assert!(temp.ends_with("°C"));
                // Temperature should be a reasonable value
                let temp_str = temp.strip_suffix("°C").unwrap();
                let temp_val: f64 = temp_str.parse().unwrap();
                assert!(temp_val > -50.0 && temp_val < 60.0);
            }
            Err(e) => {
                // Network errors are acceptable in tests
                assert!(e.contains("error") || e.contains("Unknown location"));
            }
        }
    }

    #[test]
    #[ignore] // Use `cargo test -- --ignored` to run this test
    fn test_unknown_location_integration() {
        // This test requires internet connection
        // Run with: cargo test test_unknown_location_integration -- --ignored
        let result = get_weather("NonExistentCityXYZ123");
        match result {
            Err(msg) => assert!(msg.starts_with("Unknown location:")),
            Ok(_) => panic!("Expected error for non-existent city"),
        }
    }
}
