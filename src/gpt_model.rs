#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum GptModel {
    Gpt3_5Turbo,
    Gpt4o,
    Gpt4oMini,
    Gpt4Turbo,
    Gpt5,
    OpenAio4Mini,
    OpenAio3Pro,
}

impl GptModel {
    pub fn as_str(self) -> &'static str {
        match self {
            GptModel::Gpt3_5Turbo => "gpt-3.5-turbo",
            GptModel::Gpt4o => "gpt-4o",
            GptModel::Gpt4oMini => "gpt-4o-mini",
            GptModel::Gpt4Turbo => "gpt-4-turbo",
            GptModel::Gpt5 => "gpt-5",
            GptModel::OpenAio4Mini => "openai-o4-mini",
            GptModel::OpenAio3Pro => "openai-o3-pro",
        }
    }
}

impl From<GptModel> for String {
    fn from(model: GptModel) -> Self {
        model.as_str().to_string()
    }
}

impl TryFrom<&str> for GptModel {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "gpt-3.5-turbo" => Ok(GptModel::Gpt3_5Turbo),
            "gpt-4o" => Ok(GptModel::Gpt4o),
            "gpt-4o-mini" => Ok(GptModel::Gpt4oMini),
            "gpt-4-turbo" => Ok(GptModel::Gpt4Turbo),
            "gpt-5" => Ok(GptModel::Gpt5),
            "openai-o4-mini" => Ok(GptModel::OpenAio4Mini),
            "openai-o3-mini" => Ok(GptModel::OpenAio3Pro),
            other => Err(format!("Unknown model: {other}")),
        }
    }
}
