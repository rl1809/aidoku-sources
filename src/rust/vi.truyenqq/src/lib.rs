#![no_std]
pub mod helper;
pub mod template;

use aidoku::{
	error::Result,
	prelude::*,
	std::{defaults::defaults_get, net::Request, String, StringRef, Vec},
	Chapter, DeepLink, Filter, FilterType, Listing, Manga, MangaPageResult, MangaStatus,
	MangaViewer, Page,
};
use crate::{helper::urlencode, template::WPComicsSource};

fn get_instance() -> WPComicsSource {
	WPComicsSource {
		base_url: String::from("https://truyenqqto.com"),
		viewer: MangaViewer::Rtl,
		listing_mapping: |listing| {
			String::from(match listing.as_str() {
				"Truyện con gái" => "truyen-con-gai",
				"Truyện con trai" => "truyen-con-trai",
				_ => "",
			})
		},
		status_mapping: |status| match status.trim() {
			"Đang Cập Nhật" => MangaStatus::Ongoing,
			"Hoàn Thành" => MangaStatus::Completed,
			_ => MangaStatus::Unknown,
		},
		time_converter: |ago| {
			StringRef::from(ago)
				.0
				.as_date("dd/MM/yyyy", None, None)
				.unwrap_or(-1.0)
		},

		next_page: "div.page_redirect span[aria-hidden=true]:contains(›)",
		manga_cell: "ul.list_grid li",
		manga_cell_title: "div.book_info > div.book_name > h3 > a",
		manga_cell_url: "div.book_info > div.book_name > h3 > a",
		manga_cell_image: "div.book_avatar img",
		manga_cell_image_attr: "src",

		manga_listing_pagination: "/trang-",
		manga_listing_extension: ".html",

		manga_details_title: "div.book_other h1[itemprop=name]",
		manga_details_cover: "div.book_avatar img",
		manga_details_author: "li.author.row p.col-xs-9 a",
		manga_details_description: "div.story-detail-info.detail-content",
		manga_details_tags: "ul.list01 > li a",
		manga_details_tags_splitter: "",
		manga_details_status: "li.status.row p.col-xs-9",
		manga_details_chapters: "div.works-chapter-item",

		chapter_skip_first: false,
		chapter_anchor_selector: "div.name-chap a",
        chapter_date_selector: "div.time-chap",

		page_url_transformer: |url| {
			let mut server_two = String::from("https://images2-focus-opensocial.googleusercontent.com/gadgets/proxy?container=focus&gadget=a&no_expand=1&resize_h=0&rewriteMime=image%2F*&url=");
			if let Ok(server_selection) = defaults_get("serverSelection") {
				if let Ok(2) = server_selection.as_int() {
					server_two.push_str(&urlencode(url));
					server_two
				} else {
					url
				}
			} else {
				url
			}
		},
		vinahost_protection: true,
		..Default::default()
	}
}

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
    fn get_search_url(filters: Vec<Filter>, page: i32) -> String {
        let mut excluded_tags: Vec<String> = Vec::new();
        let mut included_tags: Vec<String> = Vec::new();
        let mut query = String::new();
        for filter in filters {
            match filter.kind {
                FilterType::Title => {
                    let title = urlencode(
                        filter
                            .value
                            .as_string()
                            .unwrap_or_else(|_| StringRef::from(""))
                            .read(),
                    );
                    if !title.is_empty() {
                        return format!(
                            "https://truyenqqto.com/tim-kiem/trang-{page}.html?q={title}"
                        );
                    }
                }
                FilterType::Genre => {
                    let genre = filter
                        .object
                        .get("id")
                        .as_string()
                        .unwrap_or_else(|_| StringRef::from(""))
                        .read();
                    if genre.is_empty() {
                        continue;
                    }
                    match filter.value.as_int().unwrap_or(-1) {
                        0 => excluded_tags.push(genre),
                        1 => included_tags.push(genre),
                        _ => continue,
                    }
                }
                _ => match filter.name.as_str() {
                    "Tình trạng" => {
                        let mut status = filter.value.as_int().unwrap_or(-1);
                        if status == 0 {
                            status = -1
                        }
                        query.push_str("&status=");
                        query.push_str(format!("{}", status).as_str());
                    }
                    "Quốc gia" => {
                        let country = filter.value.as_int().unwrap_or(-1);
                        if country >= 0 {
                            query.push_str("&country=");
                            query.push_str(format!("{}", country).as_str());
                        }
                    }
                    "Số lượng chapter" => {
                        let minchapter = match filter.value.as_int().unwrap_or(-1) {
                            0 => "0",
                            1 => "50",
                            2 => "100",
                            3 => "200",
                            4 => "300",
                            5 => "400",
                            6 => "500",
                            _ => continue,
                        };
                        query.push_str("&minchapter=");
                        query.push_str(minchapter);
                    }
                    "Sắp xếp theo" => {
                        let sort = filter.value.as_int().unwrap_or(-1);
                        if sort >= 0 {
                            query.push_str("&sort=");
                            query.push_str(format!("{}", sort).as_str());
                        }
                    }
                    _ => continue,
                },
            }
        }
        format!(
            "https://truyenqqto.com/tim-kiem-nang-cao.html?category={}¬category={}{}",
            included_tags.join(","),
            excluded_tags.join(","),
            query
        )
    }
	get_instance().get_manga_list(get_search_url(filters, page))
}

#[get_manga_listing]
fn get_manga_listing(listing: Listing, page: i32) -> Result<MangaPageResult> {
	get_instance().get_manga_listing(listing, page)
}

#[get_manga_details]
fn get_manga_details(id: String) -> Result<Manga> {
    let instance = get_instance();
    let manga = instance.get_manga_details(id.clone())?;

    let mut detail_url = instance.base_url;
    detail_url.push_str(&id);

    let request = Request::get(detail_url);
    let html = request.html().unwrap();

    let description = html.select(instance.manga_details_description).text().read();

    let tags: Vec<String> = html
        .select(instance.manga_details_tags)
        .array()
        .map(|el| el.text().read())
        .collect();
	let authors: Vec<String> = html
        .select(instance.manga_details_author)
        .array()
        .map(|el| el.text().read())
        .collect();


    let status = html
        .select(instance.manga_details_status)
        .text()
        .read();
    let status = (instance.status_mapping)(status);

    Ok(Manga {
		id: manga.id,
		cover: manga.cover,
		title: manga.title,
		author: authors.join(", "),
		artist: String::new(),
		description,
		url: manga.url,
		status,
		categories: tags,
		viewer: manga.viewer,
	})
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
	let instance = get_instance();
	let mut chapter_list = Vec::new();
	let mut detail_url = instance.base_url;
	detail_url.push_str(&id);
	let request = Request::get(detail_url);
	let html = request.html()?;
	for (index, element) in html.select(instance.manga_details_chapters).array().enumerate() {
		let chapter_url = element.select(instance.chapter_anchor_selector).attr("href").read();
		let chapter_title = element.select(instance.chapter_anchor_selector).text().read();
		let chapter_date = element.select(instance.chapter_date_selector).text().read();

		let chapter_number = StringRef::from(&chapter_title).0.match_regex(r"(?:Chuong|Chương) (\d+(\.\d+)?)").and_then(|groups|{
			groups.get(1).map(|m|{
				StringRef::from(m).0.as_float().unwrap_or(0.0)
			})
		}).unwrap_or(index as f64);

		let date = (instance.time_converter)(chapter_date);
		chapter_list.push(Chapter{
			id: chapter_url,
			title: chapter_title,
			chapter: chapter_number,
			date_updated: date,
			scanlator: String::new(),
			volume: -1.0,
			lang: String::from("vi-vn"),
		})
	}
	Ok(chapter_list)
}


#[get_page_list]
fn get_page_list(_manga_id: String, chapter_id: String) -> Result<Vec<Page>> {
	get_instance().get_page_list(chapter_id)
}

#[modify_image_request]
fn modify_image_request(request: Request) {
	get_instance().modify_image_request(request)
}

#[handle_url]
fn handle_url(url: String) -> Result<DeepLink> {
	get_instance().handle_url(url)
}