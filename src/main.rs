use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use reqwest;
use std::error::Error;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use colored::customcolors::CustomColor;

#[derive(Deserialize, Serialize, Debug)]
struct UpdateInfo {
    version: String,
    url: String,
}

const UPDATE_CHECK_URL: &str = "https://howdyho.net/discord/version.php";
const MATERIAL_PAGE_URL: &str = "https://howdyho.net/windows-software/discord-fix-snova-rabotayushij-diskord-vojs-zvonki";

const ORANGE: CustomColor = CustomColor { r: 252, g: 197, b: 108 };
const GREEN: CustomColor = CustomColor { r: 126, g: 176, b: 0 };
const BLUE: CustomColor = CustomColor { r: 87, g: 170, b: 247 };
const MAGENTA: CustomColor = CustomColor { r: 196, g: 124, b: 186 };

fn compare_versions(v1: &str, v2: &str) -> bool {
    let v1_parts: Vec<u32> = v1.split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    let v2_parts: Vec<u32> = v2.split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    v1_parts > v2_parts
}

async fn download_file(client: &reqwest::Client, url: &str, pb: &ProgressBar) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut response = client.get(url).send().await?;
    let total_size = response.content_length().unwrap_or(0);
    pb.set_length(total_size);

    let mut buffer = Vec::with_capacity(total_size as usize);
    let mut downloaded = 0;

    // Get content in chunks using response.chunk()
    while let Some(chunk) = response.chunk().await? {
        buffer.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    Ok(buffer)
}

fn extract_filename_from_url(url: &str) -> String {
    // Сначала отделяем query string (всё после ?)
    let base_url = url.split('?').next().unwrap_or(url);

    // Затем берём последнюю часть пути
    let filename = base_url.split('/')
        .last()
        .unwrap_or("update.zip")
        .to_string();

    // Если по какой-то причине имя пустое, используем значение по умолчанию
    if filename.is_empty() {
        return "update.zip".to_string();
    }

    filename
}

async fn check_and_update() -> Result<(), Box<dyn Error>> {
    println!("{}", "Проверка обновлений...".bright_white());
    let response = reqwest::get(UPDATE_CHECK_URL).await?;
    let update_info: UpdateInfo = response.json().await?;

    let bin_dir = Path::new("bin");
    if !bin_dir.exists() {
        fs::create_dir_all(bin_dir)?;
    }

    let version_path = bin_dir.join("version.txt");
    let local_version = if version_path.exists() {
        fs::read_to_string(&version_path)?
    } else {
        let default_version = "5.2".to_string();
        fs::write(&version_path, &default_version)?;
        default_version
    };

    if compare_versions(&update_info.version, &local_version) {
        println!("{} {} (Текущая версия: {})!",
                 "Доступна НОВАЯ ВЕРСИЯ".custom_color(GREEN),
                 update_info.version.custom_color(GREEN).bold(),
                 local_version.bright_black()
        );
        println!("{}\n", "Загружаю...".custom_color(ORANGE));

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        let response = client.get(&update_info.url).send().await?;
        let final_url = response.url().to_string();

        // Извлекаем имя файла, игнорируя query string
        let filename = extract_filename_from_url(&final_url);

        let pb = ProgressBar::new(0);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
            .expect("Progress bar template error")
            .progress_chars("#X-"));

        let data = download_file(&client, &final_url, &pb).await?;

        // Сохраняем файл с правильным именем
        let archive_path = Path::new(&filename);
        let mut file = File::create(&archive_path)?;
        file.write_all(&data)?;

        pb.finish_with_message("Загрузка завершена".custom_color(GREEN).to_string());

        fs::write(&version_path, &update_info.version)?;

        println!("\n\n{} {}",
                 "ОБНОВЛЕНИЕ успешно загружено как:".custom_color(BLUE).bold(),
                 filename.underline()
        );
        println!("{} {}.",
                 "Откройте загруженный архив и распакуйте обновление".custom_color(MAGENTA),
                 "ВРУЧНУЮ".custom_color(MAGENTA).underline()
        );
        println!("{}",
                 "Мы не можем сделать это за вас автоматически, чтобы случайно не затереть ваши пре-конфиги/настройки которые вы вносили в свою сборку."
                     .bright_black()
        );
    } else {
        println!("{} {}",
                 "У вас установлена последняя версия:".custom_color(GREEN),
                 local_version.custom_color(GREEN).bold()
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match check_and_update().await {
        Ok(_) => println!("\n{}", "Проверка обновлений успешно завершена.".bright_black()),
        Err(e) => eprintln!("\n{} {}\n{}", "Ошибка при проверке обновлений:".red().bold(), e, format!("Перейдите по ссылке и скачайте обновление вручную: {}!", MATERIAL_PAGE_URL)),
    }

    println!("{}", "Нажмите Enter для выхода...".bright_black());
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}