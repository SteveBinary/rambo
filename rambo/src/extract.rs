use anyhow::Context;
use chrono::{DateTime, FixedOffset};
use nom_exif::{Exif, ExifIter, ExifTag, MediaParser, MediaSource, TrackInfo, TrackInfoTag};
use std::fs::File;

pub fn extract_creation_datetime_from_media_source(media_source: MediaSource<File>, media_parser: &mut MediaParser) -> anyhow::Result<DateTime<FixedOffset>> {
    if media_source.has_exif() {
        let exif_iter: ExifIter = media_parser.parse(media_source).context("Failed to parse EXIF data!")?;

        let exif: Exif = exif_iter.into();
        extract_creation_datetime_from_exif(&exif)
    } else if media_source.has_track() {
        let track_info: TrackInfo = media_parser.parse(media_source)?;
        extract_creation_datetime_from_track_info(&track_info)
    } else {
        Err(anyhow::anyhow!("The media source has no EXIF or track data!"))
    }
}

const EXIF_TAGS_FOR_CREATION_DATETIME: [ExifTag; 3] = [ExifTag::DateTimeOriginal, ExifTag::OffsetTimeOriginal, ExifTag::CreateDate];

fn extract_creation_datetime_from_exif(exif: &Exif) -> anyhow::Result<DateTime<FixedOffset>> {
    for exif_tag in EXIF_TAGS_FOR_CREATION_DATETIME {
        if let Some(exif_value) = exif.get(exif_tag) {
            if let Some(datetime) = exif_value.as_time() {
                return Ok(datetime);
            }
        }
    }

    Err(anyhow::anyhow!("Could not get the creation datetime from EXIF data!"))
}

const TRACK_INFO_TAGS_FOR_CREATION_DATETIME: [TrackInfoTag; 1] = [TrackInfoTag::CreateDate];

fn extract_creation_datetime_from_track_info(track_info: &TrackInfo) -> anyhow::Result<DateTime<FixedOffset>> {
    for track_info_tag in TRACK_INFO_TAGS_FOR_CREATION_DATETIME {
        if let Some(exif_value) = track_info.get(track_info_tag) {
            if let Some(datetime) = exif_value.as_time() {
                return Ok(datetime);
            }
        }
    }

    Err(anyhow::anyhow!("Could not get the creation datetime from track info data!"))
}
