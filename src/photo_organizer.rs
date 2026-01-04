use chrono::{DateTime, Datelike, Local};
use gag::Gag;
use rexif;
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

pub struct PhotoOrganizer {
    source: PathBuf,
    destination: PathBuf,
    use_hierarchy: bool,
}

impl PhotoOrganizer {
    pub fn new(source: String, destination: String, use_hierarchy: bool) -> Self {
        PhotoOrganizer {
            source: PathBuf::from(source),
            destination: PathBuf::from(destination),
            use_hierarchy,
        }
    }

    pub fn copy_photo(&self, photo_path: &Path) -> io::Result<()> {
        // Try to get date from EXIF data
        let date = self
            .get_exif_date(photo_path)
            .or_else(|| self.get_file_modified_date(photo_path));

        let dest_path = if self.use_hierarchy {
            if let Some(date) = date {
                // Create year/month hierarchy
                let year = date.year();
                let month = date.month();
                let month_name = match month {
                    1 =>  "01",
                    2 =>  "02",
                    3 =>  "03",
                    4 =>  "04",
                    5 =>  "05",
                    6 =>  "06",
                    7 =>  "07",
                    8 =>  "08",
                    9 =>  "09",
                    10 => "10",
                    11 => "11",
                    12 => "12",
                    _ =>  "Unknown",
                };

                let year_month_dir = self.destination.join(year.to_string()).join(month_name);
                fs::create_dir_all(&year_month_dir)?;
                year_month_dir.join(photo_path.file_name().unwrap())
            } else {
                // No date available, put in "Unknown" folder
                let unknown_dir = self.destination.join("Unknown");
                fs::create_dir_all(&unknown_dir)?;
                unknown_dir.join(photo_path.file_name().unwrap())
            }
        } else {
            // Flat structure
            fs::create_dir_all(&self.destination)?;
            self.destination.join(photo_path.file_name().unwrap())
        };

        // Handle file name conflicts
        let final_dest = self.get_unique_filename(dest_path);

        // Copy the file
        fs::copy(photo_path, &final_dest)?;

        Ok(())
    }

    fn get_exif_date(&self, photo_path: &Path) -> Option<DateTime<Local>> {
        let contents = fs::read(photo_path).ok()?;
        
        // Suppress rexif warnings/errors that are printed to stderr
        let exif = {
            let _stderr_gag = Gag::stderr().ok();
            match rexif::parse_buffer(&contents) {
                Ok(e) => e,
                Err(_) => return None,
            }
        };

        // Try different EXIF date tags
        for entry in exif.entries {
            if matches!(
                entry.tag,
                rexif::ExifTag::DateTimeOriginal
                    | rexif::ExifTag::DateTime
                    | rexif::ExifTag::DateTimeDigitized
            ) {
                if let rexif::TagValue::Ascii(ref date_str) = entry.value {
                    // EXIF date format is typically "YYYY:MM:DD HH:MM:SS"
                    if let Ok(datetime) = chrono::NaiveDateTime::parse_from_str(
                        date_str,
                        "%Y:%m:%d %H:%M:%S",
                    ) {
                        return Some(DateTime::from_naive_utc_and_offset(
                            datetime,
                            *Local::now().offset(),
                        ));
                    }
                }
            }
        }

        None
    }

    fn get_file_modified_date(&self, photo_path: &Path) -> Option<DateTime<Local>> {
        let metadata = fs::metadata(photo_path).ok()?;
        let modified = metadata.modified().ok()?;
        Some(DateTime::from(modified))
    }

    fn get_unique_filename(&self, path: PathBuf) -> PathBuf {
        if !path.exists() {
            return path;
        }

        let stem = path.file_stem().unwrap().to_string_lossy();
        let extension = path.extension().unwrap_or_default().to_string_lossy();
        let parent = path.parent().unwrap();

        for i in 1..10000 {
            let new_name = if extension.is_empty() {
                format!("{}_{}", stem, i)
            } else {
                format!("{}_{}.{}", stem, i, extension)
            };
            let new_path = parent.join(new_name);
            if !new_path.exists() {
                return new_path;
            }
        }

        path // Fallback to original if we somehow exhausted all numbers
    }
}
