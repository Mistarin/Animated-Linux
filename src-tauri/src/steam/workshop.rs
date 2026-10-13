use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub preview_url: String,
    pub author: String,
    pub votes_up: u32,
    pub file_size_mb: f64,
    pub tags: Vec<String>,
    pub is_downloaded: bool,
}

pub struct WorkshopManager;

impl WorkshopManager {
    pub fn new() -> Self {
        Self
    }

    pub fn get_community_items(&self) -> Vec<WorkshopItem> {
        // Preset community / workshop items catalogue for the user to download or try
        vec![
            WorkshopItem {
                id: "ws-101".to_string(),
                title: "Cyberpunk Rain City 4K".to_string(),
                description: "Futuristic rainy neon cityscape with subtle dynamic lighting and reflections.".to_string(),
                preview_url: "https://images.unsplash.com/photo-1518709268805-4e9042af9f23?auto=format&fit=crop&w=800&q=80".to_string(),
                author: "NeonDrifter".to_string(),
                votes_up: 1420,
                file_size_mb: 28.5,
                tags: vec!["Sci-Fi".to_string(), "Cyberpunk".to_string(), "4K".to_string()],
                is_downloaded: false,
            },
            WorkshopItem {
                id: "ws-102".to_string(),
                title: "Deep Space Nebula & Black Hole".to_string(),
                description: "Stunning volumetric cosmic nebula with glowing accretion disk.".to_string(),
                preview_url: "https://images.unsplash.com/photo-1451187580459-43490279c0fa?auto=format&fit=crop&w=800&q=80".to_string(),
                author: "AstroVFX".to_string(),
                votes_up: 3890,
                file_size_mb: 42.1,
                tags: vec!["Space".to_string(), "Relaxing".to_string(), "Cosmic".to_string()],
                is_downloaded: false,
            },
            WorkshopItem {
                id: "ws-103".to_string(),
                title: "Retro Synthwave Sunset Horizon".to_string(),
                description: "80s outrun grid with wireframe mountains and glowing chrome sun.".to_string(),
                preview_url: "https://images.unsplash.com/photo-1509198397868-475647b2a1e5?auto=format&fit=crop&w=800&q=80".to_string(),
                author: "KavinskyGrid".to_string(),
                votes_up: 2750,
                file_size_mb: 19.8,
                tags: vec!["Synthwave".to_string(), "Retro".to_string(), "Music".to_string()],
                is_downloaded: false,
            },
            WorkshopItem {
                id: "ws-104".to_string(),
                title: "Serene Cherry Blossom Waterfall".to_string(),
                description: "Tranquil Japanese garden with falling sakura petals and smooth water flow.".to_string(),
                preview_url: "https://images.unsplash.com/photo-1528164344705-475426879c0d?auto=format&fit=crop&w=800&q=80".to_string(),
                author: "ZenMaster".to_string(),
                votes_up: 5120,
                file_size_mb: 34.0,
                tags: vec!["Nature".to_string(), "Anime".to_string(), "Minimal".to_string()],
                is_downloaded: false,
            },
        ]
    }
}
