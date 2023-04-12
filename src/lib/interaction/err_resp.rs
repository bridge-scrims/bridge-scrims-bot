use serenity::builder::CreateInteractionResponseData;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub struct ErrorResponse<'a>(
    pub CreateInteractionResponseData<'a>, 
    pub String
);

pub type Result<'a, T> = std::result::Result<T, Box<ErrorResponse<'a>>>;

impl ErrorResponse<'_> {

    pub fn message<'a, S: ToString>(message: S) -> Box<ErrorResponse<'a>> {
        let mut resp = CreateInteractionResponseData::default();
        resp.embed(|e| e.description(message.to_string()).color(0xff003c));
        Box::new(ErrorResponse(resp, message.to_string()))
    }

    pub fn with_title<'a, S: ToString, T: ToString>(title: S, description: T) -> Box<ErrorResponse<'a>> {
        let mut resp = CreateInteractionResponseData::default();
        resp.embed(|e| e.title(title).description(description.to_string()).color(0xff003c));
        Box::new(ErrorResponse(resp, description.to_string()))
    }

    pub fn with_footer<'a, S: ToString, T: ToString, U: ToString>(title: S, description: T, footer: U) -> Box<ErrorResponse<'a>> {
        let mut resp = CreateInteractionResponseData::default();
        resp.embed(
            |e| 
                e.title(title).description(description.to_string())
                    .footer(|f| f.text(footer)).color(0xff003c)
        );
        Box::new(ErrorResponse(resp, description.to_string()))
    }
}

impl fmt::Display for ErrorResponse<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InteractionErrorResponse({})", self.1)
    }
}

impl StdError for ErrorResponse<'_> {}