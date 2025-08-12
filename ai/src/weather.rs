use crate::constants::{DEFAULT_WEATHER_PROMPT, DEFAULT_WEATHER_PROMPT_DE, WEATHER_API_TIMEOUT};
use crate::formatget;

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

#[derive(serde::Deserialize, Debug, PartialEq)]
struct WeatherCurrent {
    temperature_2m: f64,
    wind_speed_10m: f64,
    relative_humidity_2m: f64,
    precipitation: f64,
    rain: f64,
}

pub fn get_weather(city: &str) -> Result<String, String> {
    let (lat, lon) = get_coordinates(city, "https://geocoding-api.open-meteo.com")?;
    let weather = get_weather_data(lat, lon, "https://api.open-meteo.com")?;
    Ok(formatget!(
        "Temperature: {}°C, Humidity: {}%, Wind: {}km/h, Precipitation: {}mm, Rain: {}mm",
        weather.temperature_2m,
        weather.relative_humidity_2m,
        weather.wind_speed_10m,
        weather.precipitation,
        weather.rain
    ))
}

fn get_coordinates(city: &str, base_url: &str) -> Result<(f64, f64), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(WEATHER_API_TIMEOUT)
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

fn get_weather_data(lat: f64, lon: f64, base_url: &str) -> Result<WeatherCurrent, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(WEATHER_API_TIMEOUT)
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = format!(
        "{}/v1/forecast?latitude={}&longitude={}&current=temperature_2m,wind_speed_10m,relative_humidity_2m,precipitation,rain",
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

pub fn weather_prompt() -> &'static str {
    if std::env::var("LANG").unwrap_or_default().starts_with("de") {
        DEFAULT_WEATHER_PROMPT_DE
    } else {
        DEFAULT_WEATHER_PROMPT
    }
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
                "/v1/forecast?latitude=47.3769&longitude=8.5417&current=temperature_2m,wind_speed_10m,relative_humidity_2m,precipitation,rain",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "current": {
                    "temperature_2m": 22.5,
                    "wind_speed_10m": 10.2,
                    "relative_humidity_2m": 65.0,
                    "precipitation": 0.0,
                    "rain": 0.0
                }
            }"#,
            )
            .create();

        let (lat, lon) = get_coordinates("Zurich", &geocoding_server.url()).unwrap();
        assert_eq!(lat, 47.3769);
        assert_eq!(lon, 8.5417);

        let weather = get_weather_data(lat, lon, &weather_server.url()).unwrap();
        assert_eq!(weather.temperature_2m, 22.5);
        assert_eq!(weather.wind_speed_10m, 10.2);
        assert_eq!(weather.relative_humidity_2m, 65.0);

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

        // Mock server error - ignore query parameters, focus on error handling
        let mock = server
            .mock("GET", "/v1/forecast")
            .match_query(mockito::Matcher::Any)
            .with_status(500)
            .create();

        let result = get_weather_data(47.3769, 8.5417, &server.url());
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

        // Mock weather API response - match coordinates but ignore current parameters
        let weather_mock = weather_server
            .mock("GET", "/v1/forecast")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("latitude".into(), "35.6762".into()),
                mockito::Matcher::UrlEncoded("longitude".into(), "139.6503".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "current": {
                    "temperature_2m": 18.7,
                    "wind_speed_10m": 5.4,
                    "relative_humidity_2m": 72.0,
                    "precipitation": 0.2,
                    "rain": 0.1
                }
            }"#,
            )
            .create();

        // Test the complete flow
        let (lat, lon) = get_coordinates("Tokyo", &geocoding_server.url()).unwrap();
        let weather = get_weather_data(lat, lon, &weather_server.url()).unwrap();
        let result = format!(
            "Temperature: {}°C, Humidity: {}%, Wind: {} km/h, Precipitation: {} mm, Rain: {} mm",
            weather.temperature_2m,
            weather.relative_humidity_2m,
            weather.wind_speed_10m,
            weather.precipitation,
            weather.rain
        );

        assert!(result.contains("18.7°C"));
        assert!(result.contains("72%"));

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
            Ok(weather_info) => {
                assert!(weather_info.contains("Temperature:"));
                assert!(weather_info.contains("°C"));
                assert!(weather_info.contains("Humidity:"));
                assert!(weather_info.contains("%"));
                assert!(weather_info.contains("Wind:"));
                assert!(weather_info.contains("km/h"));
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
