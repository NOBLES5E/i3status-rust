//! Current weather
//!
//! This block displays local weather and temperature information. In order to use this block, you
//! will need access to a supported weather API service. At the time of writing, OpenWeatherMap and
//! met.no are supported.
//!
//! Configuring this block requires configuring a weather service, which may require API keys and
//! other parameters.
//!
//! If using the `autolocate` feature, set the block update interval such that you do not exceed ipapi.co's free daily limit of 1000 hits.
//!
//! # Configuration
//!
//! Key | Values | Default
//! ----|--------|--------
//! `service` | The configuration of a weather service (see below). | **Required**
//! `format` | A string to customise the output of this block. See below for available placeholders. Text may need to be escaped, refer to [Escaping Text](#escaping-text). | `"$weather $temp"`
//! `interval` | Update interval, in seconds. | `600`
//! `autolocate` | Gets your location using the ipapi.co IP location service (no API key required). If the API call fails then the block will fallback to `city_id` or `place`. | `false`
//!
//! # OpenWeatherMap Options
//!
//! To use the service you will need a (free) API key.
//!
//! Key | Values | Required | Default
//! ----|--------|----------|--------
//! `name` | `openweathermap`. | Yes | None
//! `api_key` | Your OpenWeatherMap API key. | Yes | None
//! `city_id` | OpenWeatherMap's ID for the city. | Yes* | None
//! `place` | OpenWeatherMap 'By city name' search query. See [here](https://openweathermap.org/current) | Yes* | None
//! `coordinates` | GPS latitude longitude coordinates as a tuple, example: `["39.2362","9.3317"]` | Yes* | None
//! `units` | Either `"metric"` or `"imperial"`. | No | `"metric"`
//! `lang` | Language code. See [here](https://openweathermap.org/current#multi). Currently only affects `weather_verbose` key. | No | `"en"`
//!
//! One of `city_id`, `place` or `coordinates` is required. If more than one are supplied, `city_id` takes precedence over `place` which takes place over `coordinates`.
//!
//! The options `api_key`, `city_id`, `place` can be omitted from configuration,
//! in which case they must be provided in the environment variables
//! `OPENWEATHERMAP_API_KEY`, `OPENWEATHERMAP_CITY_ID`, `OPENWEATHERMAP_PLACE`.
//!
//! # met.no Options
//!
//! Key | Values | Required | Default
//! ----|--------|----------|--------
//! `name` | `metno`. | Yes | None
//! `coordinates` | GPS latitude longitude coordinates as a tuple, example: `["39.2362","9.3317"]` | Required if `autolocate = false` | None
//! `lang` | Language code: `en`, `nn` or `nb` | No | `en`
//! `altitude` | Meters above sea level of the ground | No | Approximated by server
//!
//! Met.no does not support location name.
//!
//! # Available Format Keys
//!
//!  Key              | Value                                                              | Type   | Unit
//! ------------------|--------------------------------------------------------------------|--------|-----
//! `location`        | Location name (exact format depends on the service)                | Text   | -
//! `temp`            | Temperature                                                        | Number | degrees
//! `apparent`        | Australian Apparent Temperature                                    | Number | degrees
//! `humidity`        | Humidity                                                           | Number | %
//! `weather`         | Textual brief description of the weather, e.g. "Raining"           | Text   | -
//! `weather_verbose` | Textual verbose description of the weather, e.g. "overcast clouds" | Text   | -
//! `wind`            | Wind speed                                                         | Number | -
//! `wind_kmh`        | Wind speed. The wind speed in km/h                                 | Number | -
//! `direction`       | Wind direction, e.g. "NE"                                          | Text   | -
//!
//! # Example
//!
//! Show detailed weather in San Francisco through the OpenWeatherMap service:
//!
//! ```toml
//! [[block]]
//! block = "weather"
//! format = "$weather ($location) $temp, $wind m/s $direction"
//! [block.service]
//! name = "openweathermap"
//! api_key = "XXX"
//! city_id = "5398563"
//! units = "metric"
//! ```
//!
//! # Used Icons
//!
//! - `weather_sun` (when weather is reported as "Clear")
//! - `weather_rain` (when weather is reported as "Rain" or "Drizzle")
//! - `weather_clouds` (when weather is reported as "Clouds", "Fog" or "Mist")
//! - `weather_thunder` (when weather is reported as "Thunderstorm")
//! - `weather_snow` (when weather is reported as "Snow")
//! - `weather_default` (in all other cases)

use super::prelude::*;

mod met_no;
mod open_weather_map;

const IP_API_URL: &str = "https://ipapi.co/json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WeatherConfig {
    #[serde(default = "default_interval")]
    interval: Seconds,
    #[serde(default)]
    format: FormatConfig,
    service: WeatherService,
    #[serde(default)]
    autolocate: bool,
}

fn default_interval() -> Seconds {
    Seconds::new(600)
}

#[async_trait]
trait WeatherProvider {
    async fn get_weather(
        &self,
        autolocated_location: &Option<LocationResponse>,
    ) -> Result<WeatherResult>;
}

#[derive(Deserialize)]
#[serde(tag = "name", rename_all = "lowercase")]
enum WeatherService {
    OpenWeatherMap(open_weather_map::Config),
    MetNo(met_no::Config),
}

enum WeatherIcon {
    Sun,
    Rain,
    Clouds,
    Thunder,
    Snow,
    Default,
}

impl WeatherIcon {
    fn to_icon_str(&self) -> &str {
        match self {
            Self::Sun => "weather_sun",
            Self::Rain => "weather_rain",
            Self::Clouds => "weather_clouds",
            Self::Thunder => "weather_thunder",
            Self::Snow => "weather_snow",
            Self::Default => "weather_default",
        }
    }
}

struct WeatherResult {
    location: String,
    temp: f64,
    apparent: f64,
    humidity: f64,
    weather: String,
    weather_verbose: String,
    wind: f64,
    wind_kmh: f64,
    wind_direction: String,
    icon: WeatherIcon,
}

impl WeatherResult {
    fn into_values(self) -> HashMap<Cow<'static, str>, Value> {
        map! {
            "location" => Value::text(self.location),
            "temp" => Value::degrees(self.temp),
            "apparent" => Value::degrees(self.apparent),
            "humidity" => Value::percents(self.humidity),
            "weather" => Value::text(self.weather),
            "weather_verbose" => Value::text(self.weather_verbose),
            "wind" => Value::number(self.wind),
            "wind_kmh" => Value::number(self.wind_kmh),
            "direction" => Value::text(self.wind_direction),
        }
    }
}

pub async fn run(config: toml::Value, mut api: CommonApi) -> Result<()> {
    let config = WeatherConfig::deserialize(config).config_error()?;
    let mut widget = api
        .new_widget()
        .with_format(config.format.with_default("$weather $temp")?);

    let provider: Box<dyn WeatherProvider + Send + Sync> = match config.service {
        WeatherService::MetNo(config) => Box::new(met_no::Service::new(&mut api, config).await?),
        WeatherService::OpenWeatherMap(config) => Box::new(open_weather_map::Service::new(config)),
    };

    loop {
        let data = api
            .recoverable(|| async {
                let location = match config.autolocate {
                    true => find_ip_location().await?,
                    false => None,
                };
                provider.get_weather(&location).await
            })
            .await?;

        widget.set_icon(data.icon.to_icon_str())?;
        widget.set_values(data.into_values());
        api.set_widget(&widget).await?;

        select! {
            _ = sleep(config.interval.0) => (),
            _ = api.wait_for_update_request() => (),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, SmartDefault)]
#[serde(rename_all = "lowercase")]
enum UnitSystem {
    #[default]
    Metric,
    Imperial,
}

#[derive(Deserialize, Clone)]
struct LocationResponse {
    city: Option<String>,
    latitude: f64,
    longitude: f64,
}

impl LocationResponse {
    fn as_coordinates(&self) -> (String, String) {
        (format!("{}", self.latitude), format!("{}", self.longitude))
    }
}

// TODO: might be good to allow for different geolocation services to be used, similar to how we have `service` for the weather API
async fn find_ip_location() -> Result<Option<LocationResponse>> {
    REQWEST_CLIENT
        .get(IP_API_URL)
        .send()
        .await
        .error("Failed during request for current location")?
        .json()
        .await
        .error("Failed while parsing location API result")
}

// Convert wind direction in azimuth degrees to abbreviation names
fn convert_wind_direction(direction_opt: Option<f64>) -> &'static str {
    match direction_opt {
        Some(direction) => match direction.round() as i64 {
            24..=68 => "NE",
            69..=113 => "E",
            114..=158 => "SE",
            159..=203 => "S",
            204..=248 => "SW",
            249..=293 => "W",
            294..=338 => "NW",
            _ => "N",
        },
        None => "-",
    }
}

/// Compute the Australian Apparent Temperature from metric units
fn australian_apparent_temp(temp: f64, humidity: f64, wind_speed: f64) -> f64 {
    let exponent = 17.27 * temp / (237.7 + temp);
    let water_vapor_pressure = humidity * 0.06105 * exponent.exp();
    temp + 0.33 * water_vapor_pressure - 0.7 * wind_speed - 4.0
}
