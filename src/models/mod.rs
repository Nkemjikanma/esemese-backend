pub mod favourites;
pub use favourites::{ApiResponse, GroupImagesParams, PinataFilesResponse};

pub mod pinata;
pub use pinata::{PinataFile, PinataGroup};

pub mod groups;
pub use groups::{
    GroupCreationResponse, GroupWithThumbnail, GroupsWithThumbnailResponse, PinataGroupData,
    PinataGroupResponse,
};

pub mod uploads;
pub use uploads::{
    GroupInfo, PhotoMetadata, PhotoUpload, PinataUploadResponse, UploadResponse, UploadedFileInfo,
};

pub mod categories;
pub use categories::{CategoryParams, CategoryResponse};
