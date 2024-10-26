use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use reqwest;
use std::error::Error;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Deserialize, Serialize, Debug)]
struct UpdateInfo {
    version: String,
    url: String,
}

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

async fn check_and_update() -> Result<(), Box<dyn Error>> {
    let check_url = "https://howdyho.net/discord/version.php";

    println!("{}", "Проверка обновлений...".bright_white());
    let response = reqwest::get(check_url).await?;
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
        println!("{} {}", "Создание начального файла версии:".blue(), default_version);
        fs::write(&version_path, &default_version)?;
        default_version
    };

    if compare_versions(&update_info.version, &local_version) {
        println!("{} {} (Текущая версия: {})!",
                 "Доступна новая версия".green(),
                 update_info.version.green().bold(),
                 local_version.bright_black()
        );
        println!("{}\n", "Загружаю...".yellow());

        // Create a client that follows redirects
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        // First request to get the final URL after redirects
        let response = client.get(&update_info.url).send().await?;
        let final_url = response.url().to_string();

        // Get filename from final URL
        let file_name = final_url.split('/')
            .last()
            .ok_or("Не удалось извлечь имя файла из URL")?
            .to_string();

        // Create progress bar
        let pb = ProgressBar::new(0);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
            .expect("Progress bar template error")
            .progress_chars("#X-"));

        // Download file with progress
        let data = download_file(&client, &final_url, &pb).await?;

        // Save the file
        let archive_path = Path::new(&file_name);
        let mut file = File::create(&archive_path)?;
        file.write_all(&data)?;

        pb.finish_with_message("Загрузка завершена".green().to_string());

        fs::write(&version_path, &update_info.version)?;

        println!("\n\n{} {}", "ОБНОВЛЕНИЕ успешно загружено как:".blue().bold(), file_name.underline());
        println!("{} {}.", "Откройте загруженный архив и распакуйте обновление".cyan(), "ВРУЧНУЮ".cyan().underline());
        println!("{}", "Мы не можем сделать это за вас автоматически, чтобы случайно не затереть ваши пре-конфиги/настройки которые вы вносили в свою сборку.".bright_black());
    } else {
        println!("{} {}",
                 "У вас установлена последняя версия:".green(),
                 local_version.green().bold()
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match check_and_update().await {
        Ok(_) => println!("\n{}", "Проверка обновлений успешно завершена.".bright_black()),
        Err(e) => eprintln!("\n{} {}", "Ошибка при проверке обновлений:".red().bold(), e),
    }

    println!("{}", "Нажмите Enter для выхода...".bright_black());
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}