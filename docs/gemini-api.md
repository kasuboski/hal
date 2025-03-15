Title: Generating content  |  Gemini API  |  Google AI for Developers

URL Source: https://ai.google.dev/api/generate-content

Markdown Content:
Skip to main content
Models
More
/
Sign in
Gemini API docs
API Reference
SDKs
Pricing
Cookbook
Overview
API versions
Capabilities
Models
Generating content
Tokens
Files
Caching
Embeddings
Tuning
Semantic retrieval
All methods
Deprecated
Native image generation with Gemini 2.0 Flash Experimental is now available! Learn more
Home
Gemini API
Models
API Reference
Was this helpful?
Send feedback
Generating content
On this page
Method: models.generateContent
Endpoint
Path parameters
Request body
Example request

The Gemini API supports content generation with images, audio, code, tools, and more. For details on each of these features, read on and check out the task-focused sample code, or read the comprehensive guides.

Text generation
Vision
Audio
Long context
Code execution
JSON Mode
Function calling
System instructions
Method: models.generateContent

Generates a model response given an input GenerateContentRequest. Refer to the text generation guide for detailed usage information. Input capabilities differ between models, including tuned models. Refer to the model guide and tuning guide for details.

Endpoint
POST
https://generativelanguage.googleapis.com/v1beta/{model=models/*}:generateContent


Path parameters
model
string

Required. The name of the Model to use for generating the completion.

Format: models/{model}. It takes the form models/{model}.

Request body

The request body contains data with the following structure:

Fields
contents[]
object (Content)

Required. The content of the current conversation with the model.

For single-turn queries, this is a single instance. For multi-turn queries like chat, this is a repeated field that contains the conversation history and the latest request.

tools[]
object (Tool)

Optional. A list of Tools the Model may use to generate the next response.

A Tool is a piece of code that enables the system to interact with external systems to perform an action, or set of actions, outside of knowledge and scope of the Model. Supported Tools are Function and codeExecution. Refer to the Function calling and the Code execution guides to learn more.

toolConfig
object (ToolConfig)

Optional. Tool configuration for any Tool specified in the request. Refer to the Function calling guide for a usage example.

safetySettings[]
object (SafetySetting)

Optional. A list of unique SafetySetting instances for blocking unsafe content.

This will be enforced on the GenerateContentRequest.contents and GenerateContentResponse.candidates. There should not be more than one setting for each SafetyCategory type. The API will block any contents and responses that fail to meet the thresholds set by these settings. This list overrides the default settings for each SafetyCategory specified in the safetySettings. If there is no SafetySetting for a given SafetyCategory provided in the list, the API will use the default safety setting for that category. Harm categories HARM_CATEGORY_HATE_SPEECH, HARM_CATEGORY_SEXUALLY_EXPLICIT, HARM_CATEGORY_DANGEROUS_CONTENT, HARM_CATEGORY_HARASSMENT, HARM_CATEGORY_CIVIC_INTEGRITY are supported. Refer to the guide for detailed information on available safety settings. Also refer to the Safety guidance to learn how to incorporate safety considerations in your AI applications.

systemInstruction
object (Content)

Optional. Developer set system instruction(s). Currently, text only.

generationConfig
object (GenerationConfig)

Optional. Configuration options for model generation and outputs.

cachedContent
string

Optional. The name of the content cached to use as context to serve the prediction. Format: cachedContents/{cachedContent}

Example request
Text
Image
Audio
Video
PDF
Chat
Cache
More
Python
Node.js
Go
Shell
Kotlin
Swift
Dart
Java
from google import genai

client = genai.Client()
response = client.models.generate_content(
    model="gemini-2.0-flash", contents="Write a story about a magic backpack."
)
print(response.text)
text_generation.py
Response body

If successful, the response body contains an instance of GenerateContentResponse.

Method: models.streamGenerateContent

Generates a streamed response from the model given an input GenerateContentRequest.

Endpoint
POST
https://generativelanguage.googleapis.com/v1beta/{model=models/*}:streamGenerateContent


Path parameters
model
string

Required. The name of the Model to use for generating the completion.

Format: models/{model}. It takes the form models/{model}.

Request body

The request body contains data with the following structure:

Fields
contents[]
object (Content)

Required. The content of the current conversation with the model.

For single-turn queries, this is a single instance. For multi-turn queries like chat, this is a repeated field that contains the conversation history and the latest request.

tools[]
object (Tool)

Optional. A list of Tools the Model may use to generate the next response.

A Tool is a piece of code that enables the system to interact with external systems to perform an action, or set of actions, outside of knowledge and scope of the Model. Supported Tools are Function and codeExecution. Refer to the Function calling and the Code execution guides to learn more.

toolConfig
object (ToolConfig)

Optional. Tool configuration for any Tool specified in the request. Refer to the Function calling guide for a usage example.

safetySettings[]
object (SafetySetting)

Optional. A list of unique SafetySetting instances for blocking unsafe content.

This will be enforced on the GenerateContentRequest.contents and GenerateContentResponse.candidates. There should not be more than one setting for each SafetyCategory type. The API will block any contents and responses that fail to meet the thresholds set by these settings. This list overrides the default settings for each SafetyCategory specified in the safetySettings. If there is no SafetySetting for a given SafetyCategory provided in the list, the API will use the default safety setting for that category. Harm categories HARM_CATEGORY_HATE_SPEECH, HARM_CATEGORY_SEXUALLY_EXPLICIT, HARM_CATEGORY_DANGEROUS_CONTENT, HARM_CATEGORY_HARASSMENT, HARM_CATEGORY_CIVIC_INTEGRITY are supported. Refer to the guide for detailed information on available safety settings. Also refer to the Safety guidance to learn how to incorporate safety considerations in your AI applications.

systemInstruction
object (Content)

Optional. Developer set system instruction(s). Currently, text only.

generationConfig
object (GenerationConfig)

Optional. Configuration options for model generation and outputs.

cachedContent
string

Optional. The name of the content cached to use as context to serve the prediction. Format: cachedContents/{cachedContent}

Example request
Text
Image
Audio
Video
PDF
Chat
Python
Node.js
Go
Shell
Kotlin
Swift
Dart
Java
from google import genai

client = genai.Client()
response = client.models.generate_content_stream(
    model="gemini-2.0-flash", contents="Write a story about a magic backpack."
)
for chunk in response:
    print(chunk.text)
    print("_" * 80)
text_generation.py
Response body

If successful, the response body contains a stream of GenerateContentResponse instances.

GenerateContentResponse

Response from the model supporting multiple candidate responses.

Safety ratings and content filtering are reported for both prompt in GenerateContentResponse.prompt_feedback and for each candidate in finishReason and in safetyRatings. The API: - Returns either all requested candidates or none of them - Returns no candidates at all only if there was something wrong with the prompt (check promptFeedback) - Reports feedback on each candidate in finishReason and safetyRatings.

Fields
candidates[]
object (Candidate)

Candidate responses from the model.

promptFeedback
object (PromptFeedback)

Returns the prompt's feedback related to the content filters.

usageMetadata
object (UsageMetadata)

Output only. Metadata on the generation requests' token usage.

modelVersion
string

Output only. The model version used to generate the response.

JSON representation

{
  "candidates": [
    {
      object (Candidate)
    }
  ],
  "promptFeedback": {
    object (PromptFeedback)
  },
  "usageMetadata": {
    object (UsageMetadata)
  },
  "modelVersion": string
}
PromptFeedback

A set of the feedback metadata the prompt specified in GenerateContentRequest.content.

Fields
blockReason
enum (BlockReason)

Optional. If set, the prompt was blocked and no candidates are returned. Rephrase the prompt.

safetyRatings[]
object (SafetyRating)

Ratings for safety of the prompt. There is at most one rating per category.

JSON representation

{
  "blockReason": enum (BlockReason),
  "safetyRatings": [
    {
      object (SafetyRating)
    }
  ]
}
BlockReason

Specifies the reason why the prompt was blocked.

Enums
BLOCK_REASON_UNSPECIFIED	Default value. This value is unused.
SAFETY	Prompt was blocked due to safety reasons. Inspect safetyRatings to understand which safety category blocked it.
OTHER	Prompt was blocked due to unknown reasons.
BLOCKLIST	Prompt was blocked due to the terms which are included from the terminology blocklist.
PROHIBITED_CONTENT	Prompt was blocked due to prohibited content.
IMAGE_SAFETY	Candidates blocked due to unsafe image generation content.
UsageMetadata

Metadata on the generation request's token usage.

Fields
promptTokenCount
integer

Number of tokens in the prompt. When cachedContent is set, this is still the total effective prompt size meaning this includes the number of tokens in the cached content.

cachedContentTokenCount
integer

Number of tokens in the cached part of the prompt (the cached content)

candidatesTokenCount
integer

Total number of tokens across all the generated response candidates.

toolUsePromptTokenCount
integer

Output only. Number of tokens present in tool-use prompt(s).

thoughtsTokenCount
integer

Output only. Number of tokens of thoughts for thinking models.

totalTokenCount
integer

Total token count for the generation request (prompt + response candidates).

promptTokensDetails[]
object (ModalityTokenCount)

Output only. List of modalities that were processed in the request input.

cacheTokensDetails[]
object (ModalityTokenCount)

Output only. List of modalities of the cached content in the request input.

candidatesTokensDetails[]
object (ModalityTokenCount)

Output only. List of modalities that were returned in the response.

toolUsePromptTokensDetails[]
object (ModalityTokenCount)

Output only. List of modalities that were processed for tool-use request inputs.

JSON representation

{
  "promptTokenCount": integer,
  "cachedContentTokenCount": integer,
  "candidatesTokenCount": integer,
  "toolUsePromptTokenCount": integer,
  "thoughtsTokenCount": integer,
  "totalTokenCount": integer,
  "promptTokensDetails": [
    {
      object (ModalityTokenCount)
    }
  ],
  "cacheTokensDetails": [
    {
      object (ModalityTokenCount)
    }
  ],
  "candidatesTokensDetails": [
    {
      object (ModalityTokenCount)
    }
  ],
  "toolUsePromptTokensDetails": [
    {
      object (ModalityTokenCount)
    }
  ]
}
Candidate

A response candidate generated from the model.

Fields
content
object (Content)

Output only. Generated content returned from the model.

finishReason
enum (FinishReason)

Optional. Output only. The reason why the model stopped generating tokens.

If empty, the model has not stopped generating tokens.

safetyRatings[]
object (SafetyRating)

List of ratings for the safety of a response candidate.

There is at most one rating per category.

citationMetadata
object (CitationMetadata)

Output only. Citation information for model-generated candidate.

This field may be populated with recitation information for any text included in the content. These are passages that are "recited" from copyrighted material in the foundational LLM's training data.

tokenCount
integer

Output only. Token count for this candidate.

groundingAttributions[]
object (GroundingAttribution)

Output only. Attribution information for sources that contributed to a grounded answer.

This field is populated for GenerateAnswer calls.

groundingMetadata
object (GroundingMetadata)

Output only. Grounding metadata for the candidate.

This field is populated for GenerateContent calls.

avgLogprobs
number

Output only. Average log probability score of the candidate.

logprobsResult
object (LogprobsResult)

Output only. Log-likelihood scores for the response tokens and top tokens

index
integer

Output only. Index of the candidate in the list of response candidates.

JSON representation

{
  "content": {
    object (Content)
  },
  "finishReason": enum (FinishReason),
  "safetyRatings": [
    {
      object (SafetyRating)
    }
  ],
  "citationMetadata": {
    object (CitationMetadata)
  },
  "tokenCount": integer,
  "groundingAttributions": [
    {
      object (GroundingAttribution)
    }
  ],
  "groundingMetadata": {
    object (GroundingMetadata)
  },
  "avgLogprobs": number,
  "logprobsResult": {
    object (LogprobsResult)
  },
  "index": integer
}
FinishReason

Defines the reason why the model stopped generating tokens.

Enums
FINISH_REASON_UNSPECIFIED	Default value. This value is unused.
STOP	Natural stop point of the model or provided stop sequence.
MAX_TOKENS	The maximum number of tokens as specified in the request was reached.
SAFETY	The response candidate content was flagged for safety reasons.
RECITATION	The response candidate content was flagged for recitation reasons.
LANGUAGE	The response candidate content was flagged for using an unsupported language.
OTHER	Unknown reason.
BLOCKLIST	Token generation stopped because the content contains forbidden terms.
PROHIBITED_CONTENT	Token generation stopped for potentially containing prohibited content.
SPII	Token generation stopped because the content potentially contains Sensitive Personally Identifiable Information (SPII).
MALFORMED_FUNCTION_CALL	The function call generated by the model is invalid.
IMAGE_SAFETY	Token generation stopped because generated images contain safety violations.
GroundingAttribution

Attribution for a source that contributed to an answer.

Fields
sourceId
object (AttributionSourceId)

Output only. Identifier for the source contributing to this attribution.

content
object (Content)

Grounding source content that makes up this attribution.

JSON representation

{
  "sourceId": {
    object (AttributionSourceId)
  },
  "content": {
    object (Content)
  }
}
AttributionSourceId

Identifier for the source contributing to this attribution.

Fields
source
Union type
source can be only one of the following:
groundingPassage
object (GroundingPassageId)

Identifier for an inline passage.

semanticRetrieverChunk
object (SemanticRetrieverChunk)

Identifier for a Chunk fetched via Semantic Retriever.

JSON representation

{

  // source
  "groundingPassage": {
    object (GroundingPassageId)
  },
  "semanticRetrieverChunk": {
    object (SemanticRetrieverChunk)
  }
  // Union type
}
GroundingPassageId

Identifier for a part within a GroundingPassage.

Fields
passageId
string

Output only. ID of the passage matching the GenerateAnswerRequest's GroundingPassage.id.

partIndex
integer

Output only. Index of the part within the GenerateAnswerRequest's GroundingPassage.content.

JSON representation

{
  "passageId": string,
  "partIndex": integer
}
SemanticRetrieverChunk

Identifier for a Chunk retrieved via Semantic Retriever specified in the GenerateAnswerRequest using SemanticRetrieverConfig.

Fields
source
string

Output only. Name of the source matching the request's SemanticRetrieverConfig.source. Example: corpora/123 or corpora/123/documents/abc

chunk
string

Output only. Name of the Chunk containing the attributed text. Example: corpora/123/documents/abc/chunks/xyz

JSON representation

{
  "source": string,
  "chunk": string
}
GroundingMetadata

Metadata returned to client when grounding is enabled.

Fields
groundingChunks[]
object (GroundingChunk)

List of supporting references retrieved from specified grounding source.

groundingSupports[]
object (GroundingSupport)

List of grounding support.

webSearchQueries[]
string

Web search queries for the following-up web search.

searchEntryPoint
object (SearchEntryPoint)

Optional. Google search entry for the following-up web searches.

retrievalMetadata
object (RetrievalMetadata)

Metadata related to retrieval in the grounding flow.

JSON representation

{
  "groundingChunks": [
    {
      object (GroundingChunk)
    }
  ],
  "groundingSupports": [
    {
      object (GroundingSupport)
    }
  ],
  "webSearchQueries": [
    string
  ],
  "searchEntryPoint": {
    object (SearchEntryPoint)
  },
  "retrievalMetadata": {
    object (RetrievalMetadata)
  }
}
SearchEntryPoint

Google search entry point.

Fields
renderedContent
string

Optional. Web content snippet that can be embedded in a web page or an app webview.

sdkBlob
string (bytes format)

Optional. Base64 encoded JSON representing array of <search term, search url> tuple.

A base64-encoded string.

JSON representation

{
  "renderedContent": string,
  "sdkBlob": string
}
GroundingChunk

Grounding chunk.

Fields
chunk_type
Union type
Chunk type. chunk_type can be only one of the following:
web
object (Web)

Grounding chunk from the web.

JSON representation

{

  // chunk_type
  "web": {
    object (Web)
  }
  // Union type
}
Web

Chunk from the web.

Fields
uri
string

URI reference of the chunk.

title
string

Title of the chunk.

JSON representation

{
  "uri": string,
  "title": string
}
GroundingSupport

Grounding support.

Fields
groundingChunkIndices[]
integer

A list of indices (into 'grounding_chunk') specifying the citations associated with the claim. For instance [1,3,4] means that grounding_chunk[1], grounding_chunk[3], grounding_chunk[4] are the retrieved content attributed to the claim.

confidenceScores[]
number

Confidence score of the support references. Ranges from 0 to 1. 1 is the most confident. This list must have the same size as the groundingChunkIndices.

segment
object (Segment)

Segment of the content this support belongs to.

JSON representation

{
  "groundingChunkIndices": [
    integer
  ],
  "confidenceScores": [
    number
  ],
  "segment": {
    object (Segment)
  }
}
Segment

Segment of the content.

Fields
partIndex
integer

Output only. The index of a Part object within its parent Content object.

startIndex
integer

Output only. Start index in the given Part, measured in bytes. Offset from the start of the Part, inclusive, starting at zero.

endIndex
integer

Output only. End index in the given Part, measured in bytes. Offset from the start of the Part, exclusive, starting at zero.

text
string

Output only. The text corresponding to the segment from the response.

JSON representation

{
  "partIndex": integer,
  "startIndex": integer,
  "endIndex": integer,
  "text": string
}
RetrievalMetadata

Metadata related to retrieval in the grounding flow.

Fields
googleSearchDynamicRetrievalScore
number

Optional. Score indicating how likely information from google search could help answer the prompt. The score is in the range [0, 1], where 0 is the least likely and 1 is the most likely. This score is only populated when google search grounding and dynamic retrieval is enabled. It will be compared to the threshold to determine whether to trigger google search.

JSON representation

{
  "googleSearchDynamicRetrievalScore": number
}
LogprobsResult

Logprobs Result

Fields
topCandidates[]
object (TopCandidates)

Length = total number of decoding steps.

chosenCandidates[]
object (Candidate)

Length = total number of decoding steps. The chosen candidates may or may not be in topCandidates.

JSON representation

{
  "topCandidates": [
    {
      object (TopCandidates)
    }
  ],
  "chosenCandidates": [
    {
      object (Candidate)
    }
  ]
}
TopCandidates

Candidates with top log probabilities at each decoding step.

Fields
candidates[]
object (Candidate)

Sorted by log probability in descending order.

JSON representation

{
  "candidates": [
    {
      object (Candidate)
    }
  ]
}
Candidate

Candidate for the logprobs token and score.

Fields
token
string

The candidate’s token string value.

tokenId
integer

The candidate’s token id value.

logProbability
number

The candidate's log probability.

JSON representation

{
  "token": string,
  "tokenId": integer,
  "logProbability": number
}
CitationMetadata

A collection of source attributions for a piece of content.

Fields
citationSources[]
object (CitationSource)

Citations to sources for a specific response.

JSON representation

{
  "citationSources": [
    {
      object (CitationSource)
    }
  ]
}
CitationSource

A citation to a source for a portion of a specific response.

Fields
startIndex
integer

Optional. Start of segment of the response that is attributed to this source.

Index indicates the start of the segment, measured in bytes.

endIndex
integer

Optional. End of the attributed segment, exclusive.

uri
string

Optional. URI that is attributed as a source for a portion of the text.

license
string

Optional. License for the GitHub project that is attributed as a source for segment.

License info is required for code citations.

JSON representation

{
  "startIndex": integer,
  "endIndex": integer,
  "uri": string,
  "license": string
}
GenerationConfig

Configuration options for model generation and outputs. Not all parameters are configurable for every model.

Fields
stopSequences[]
string

Optional. The set of character sequences (up to 5) that will stop output generation. If specified, the API will stop at the first appearance of a stop_sequence. The stop sequence will not be included as part of the response.

responseMimeType
string

Optional. MIME type of the generated candidate text. Supported MIME types are: text/plain: (default) Text output. application/json: JSON response in the response candidates. text/x.enum: ENUM as a string response in the response candidates. Refer to the docs for a list of all supported text MIME types.

responseSchema
object (Schema)

Optional. Output schema of the generated candidate text. Schemas must be a subset of the OpenAPI schema and can be objects, primitives or arrays.

If set, a compatible responseMimeType must also be set. Compatible MIME types: application/json: Schema for JSON response. Refer to the JSON text generation guide for more details.

responseModalities[]
enum (Modality)

Optional. The requested modalities of the response. Represents the set of modalities that the model can return, and should be expected in the response. This is an exact match to the modalities of the response.

A model may have multiple combinations of supported modalities. If the requested modalities do not match any of the supported combinations, an error will be returned.

An empty list is equivalent to requesting only text.

candidateCount
integer

Optional. Number of generated responses to return. If unset, this will default to 1. Please note that this doesn't work for previous generation models (Gemini 1.0 family)

maxOutputTokens
integer

Optional. The maximum number of tokens to include in a response candidate.

Note: The default value varies by model, see the Model.output_token_limit attribute of the Model returned from the getModel function.

temperature
number

Optional. Controls the randomness of the output.

Note: The default value varies by model, see the Model.temperature attribute of the Model returned from the getModel function.

Values can range from [0.0, 2.0].

topP
number

Optional. The maximum cumulative probability of tokens to consider when sampling.

The model uses combined Top-k and Top-p (nucleus) sampling.

Tokens are sorted based on their assigned probabilities so that only the most likely tokens are considered. Top-k sampling directly limits the maximum number of tokens to consider, while Nucleus sampling limits the number of tokens based on the cumulative probability.

Note: The default value varies by Model and is specified by theModel.top_p attribute returned from the getModel function. An empty topK attribute indicates that the model doesn't apply top-k sampling and doesn't allow setting topK on requests.

topK
integer

Optional. The maximum number of tokens to consider when sampling.

Gemini models use Top-p (nucleus) sampling or a combination of Top-k and nucleus sampling. Top-k sampling considers the set of topK most probable tokens. Models running with nucleus sampling don't allow topK setting.

Note: The default value varies by Model and is specified by theModel.top_p attribute returned from the getModel function. An empty topK attribute indicates that the model doesn't apply top-k sampling and doesn't allow setting topK on requests.

seed
integer

Optional. Seed used in decoding. If not set, the request uses a randomly generated seed.

presencePenalty
number

Optional. Presence penalty applied to the next token's logprobs if the token has already been seen in the response.

This penalty is binary on/off and not dependant on the number of times the token is used (after the first). Use frequencyPenalty for a penalty that increases with each use.

A positive penalty will discourage the use of tokens that have already been used in the response, increasing the vocabulary.

A negative penalty will encourage the use of tokens that have already been used in the response, decreasing the vocabulary.

frequencyPenalty
number

Optional. Frequency penalty applied to the next token's logprobs, multiplied by the number of times each token has been seen in the respponse so far.

A positive penalty will discourage the use of tokens that have already been used, proportional to the number of times the token has been used: The more a token is used, the more difficult it is for the model to use that token again increasing the vocabulary of responses.

Caution: A negative penalty will encourage the model to reuse tokens proportional to the number of times the token has been used. Small negative values will reduce the vocabulary of a response. Larger negative values will cause the model to start repeating a common token until it hits the maxOutputTokens limit.

responseLogprobs
boolean

Optional. If true, export the logprobs results in response.

logprobs
integer

Optional. Only valid if responseLogprobs=True. This sets the number of top logprobs to return at each decoding step in the Candidate.logprobs_result.

enableEnhancedCivicAnswers
boolean

Optional. Enables enhanced civic answers. It may not be available for all models.

speechConfig
object (SpeechConfig)

Optional. The speech generation config.

mediaResolution
enum (MediaResolution)

Optional. If specified, the media resolution specified will be used.

JSON representation

{
  "stopSequences": [
    string
  ],
  "responseMimeType": string,
  "responseSchema": {
    object (Schema)
  },
  "responseModalities": [
    enum (Modality)
  ],
  "candidateCount": integer,
  "maxOutputTokens": integer,
  "temperature": number,
  "topP": number,
  "topK": integer,
  "seed": integer,
  "presencePenalty": number,
  "frequencyPenalty": number,
  "responseLogprobs": boolean,
  "logprobs": integer,
  "enableEnhancedCivicAnswers": boolean,
  "speechConfig": {
    object (SpeechConfig)
  },
  "mediaResolution": enum (MediaResolution)
}
Modality

Supported modalities of the response.

Enums
MODALITY_UNSPECIFIED	Default value.
TEXT	Indicates the model should return text.
IMAGE	Indicates the model should return images.
AUDIO	Indicates the model should return audio.
SpeechConfig

The speech generation config.

Fields
voiceConfig
object (VoiceConfig)

The configuration for the speaker to use.

JSON representation

{
  "voiceConfig": {
    object (VoiceConfig)
  }
}
VoiceConfig

The configuration for the voice to use.

Fields
voice_config
Union type
The configuration for the speaker to use. voice_config can be only one of the following:
prebuiltVoiceConfig
object (PrebuiltVoiceConfig)

The configuration for the prebuilt voice to use.

JSON representation

{

  // voice_config
  "prebuiltVoiceConfig": {
    object (PrebuiltVoiceConfig)
  }
  // Union type
}
PrebuiltVoiceConfig

The configuration for the prebuilt speaker to use.

Fields
voiceName
string

The name of the preset voice to use.

JSON representation

{
  "voiceName": string
}
MediaResolution

Media resolution for the input media.

Enums
MEDIA_RESOLUTION_UNSPECIFIED	Media resolution has not been set.
MEDIA_RESOLUTION_LOW	Media resolution set to low (64 tokens).
MEDIA_RESOLUTION_MEDIUM	Media resolution set to medium (256 tokens).
MEDIA_RESOLUTION_HIGH	Media resolution set to high (zoomed reframing with 256 tokens).
HarmCategory

The category of a rating.

These categories cover various kinds of harms that developers may wish to adjust.

Enums
HARM_CATEGORY_UNSPECIFIED	Category is unspecified.
HARM_CATEGORY_DEROGATORY	PaLM - Negative or harmful comments targeting identity and/or protected attribute.
HARM_CATEGORY_TOXICITY	PaLM - Content that is rude, disrespectful, or profane.
HARM_CATEGORY_VIOLENCE	PaLM - Describes scenarios depicting violence against an individual or group, or general descriptions of gore.
HARM_CATEGORY_SEXUAL	PaLM - Contains references to sexual acts or other lewd content.
HARM_CATEGORY_MEDICAL	PaLM - Promotes unchecked medical advice.
HARM_CATEGORY_DANGEROUS	PaLM - Dangerous content that promotes, facilitates, or encourages harmful acts.
HARM_CATEGORY_HARASSMENT	Gemini - Harassment content.
HARM_CATEGORY_HATE_SPEECH	Gemini - Hate speech and content.
HARM_CATEGORY_SEXUALLY_EXPLICIT	Gemini - Sexually explicit content.
HARM_CATEGORY_DANGEROUS_CONTENT	Gemini - Dangerous content.
HARM_CATEGORY_CIVIC_INTEGRITY	Gemini - Content that may be used to harm civic integrity.
ModalityTokenCount

Represents token counting info for a single modality.

Fields
modality
enum (Modality)

The modality associated with this token count.

tokenCount
integer

Number of tokens.

JSON representation

{
  "modality": enum (Modality),
  "tokenCount": integer
}
Modality

Content Part modality

Enums
MODALITY_UNSPECIFIED	Unspecified modality.
TEXT	Plain text.
IMAGE	Image.
VIDEO	Video.
AUDIO	Audio.
DOCUMENT	Document, e.g. PDF.
SafetyRating

Safety rating for a piece of content.

The safety rating contains the category of harm and the harm probability level in that category for a piece of content. Content is classified for safety across a number of harm categories and the probability of the harm classification is included here.

Fields
category
enum (HarmCategory)

Required. The category for this rating.

probability
enum (HarmProbability)

Required. The probability of harm for this content.

blocked
boolean

Was this content blocked because of this rating?

JSON representation

{
  "category": enum (HarmCategory),
  "probability": enum (HarmProbability),
  "blocked": boolean
}
HarmProbability

The probability that a piece of content is harmful.

The classification system gives the probability of the content being unsafe. This does not indicate the severity of harm for a piece of content.

Enums
HARM_PROBABILITY_UNSPECIFIED	Probability is unspecified.
NEGLIGIBLE	Content has a negligible chance of being unsafe.
LOW	Content has a low chance of being unsafe.
MEDIUM	Content has a medium chance of being unsafe.
HIGH	Content has a high chance of being unsafe.
SafetySetting

Safety setting, affecting the safety-blocking behavior.

Passing a safety setting for a category changes the allowed probability that content is blocked.

Fields
category
enum (HarmCategory)

Required. The category for this setting.

threshold
enum (HarmBlockThreshold)

Required. Controls the probability threshold at which harm is blocked.

JSON representation

{
  "category": enum (HarmCategory),
  "threshold": enum (HarmBlockThreshold)
}
HarmBlockThreshold

Block at and beyond a specified harm probability.

Enums
HARM_BLOCK_THRESHOLD_UNSPECIFIED	Threshold is unspecified.
BLOCK_LOW_AND_ABOVE	Content with NEGLIGIBLE will be allowed.
BLOCK_MEDIUM_AND_ABOVE	Content with NEGLIGIBLE and LOW will be allowed.
BLOCK_ONLY_HIGH	Content with NEGLIGIBLE, LOW, and MEDIUM will be allowed.
BLOCK_NONE	All content will be allowed.
OFF	Turn off the safety filter.
Was this helpful?
Send feedback

Except as otherwise noted, the content of this page is licensed under the Creative Commons Attribution 4.0 License, and code samples are licensed under the Apache 2.0 License. For details, see the Google Developers Site Policies. Java is a registered trademark of Oracle and/or its affiliates.

Last updated 2025-03-06 UTC.

Terms
Privacy
---
Title: Embeddings  |  Gemini API  |  Google AI for Developers

URL Source: https://ai.google.dev/api/embeddings

Markdown Content:
Embeddings  |  Gemini API  |  Google AI for Developers
=============== 

[Skip to main content](https://ai.google.dev/api/embeddings#main-content)

 [![Image 1: Google AI for Developers](https://www.gstatic.com/devrel-devsite/prod/v6bfb74446ce17cd0d3af9b93bf26e056161cb79c5a6475bd6a9c25286fcb7861/googledevai/images/lockup-new.svg)](https://ai.google.dev/)

[Models](https://ai.google.dev/gemini-api/docs)

*   Gemini
*   [About](https://deepmind.google/gemini)
*   [Docs](https://ai.google.dev/gemini-api/docs)
*   [API reference](https://ai.google.dev/api)
*   [Pricing](https://ai.google.dev/pricing)

*   Gemma
*   [About](https://ai.google.dev/gemma)
*   [Docs](https://ai.google.dev/gemma/docs)
*   [Gemmaverse](https://ai.google.dev/gemma/gemmaverse)

More

Solutions

*   Build with Gemini
*   [Gemini API](https://ai.google.dev/gemini-api/docs)
*   [Google AI Studio](https://aistudio.google.com/)

*   Customize Gemma open models
*   [Gemma open models](https://ai.google.dev/gemma)
*   [Multi-framework with Keras](https://keras.io/keras_3/)
*   [Fine-tune in Colab](https://colab.sandbox.google.com/github/google/generative-ai-docs/blob/main/site/en/gemma/docs/lora_tuning.ipynb)

*   Run on-device
*   [Google AI Edge](https://ai.google.dev/edge)
*   [Gemini Nano on Android](https://developer.android.com/ai/gemini-nano)
*   [Chrome built-in web APIs](https://developer.chrome.com/docs/ai/built-in)

*   Build responsibly
*   [Responsible GenAI Toolkit](https://ai.google.dev/responsible)
*   [Secure AI Framework](https://saif.google/)

Code assistance

*   [Android Studio](https://developer.android.com/gemini-in-android)
*   [Chrome DevTools](https://developer.chrome.com/docs/devtools/console/understand-messages)
*   [Colab](https://colab.google/)
*   [Firebase](https://firebase.google.com/products/generative-ai)
*   [Google Cloud](https://cloud.google.com/products/gemini/code-assist)
*   [JetBrains](https://plugins.jetbrains.com/plugin/8079-google-cloud-code)
*   [Jules](https://labs.google.com/jules/home)
*   [Project IDX](https://developers.google.com/idx/guides/code-with-gemini-in-idx)
*   [VS Code](https://marketplace.visualstudio.com/items?itemName=GoogleCloudTools.cloudcode)

Showcase

*   [Gemini Showcase](https://ai.google.dev/showcase)
*   [Gemini API Developer Competition](https://ai.google.dev/competition)

Community

*   [Google AI Forum](https://discuss.ai.google.dev/)
*   [Gemini for Research](https://ai.google.dev/gemini-api/docs/gemini-for-research)

/

*   [English](https://ai.google.dev/api/embeddings)
*   [Deutsch](https://ai.google.dev/api/embeddings?hl=de)
*   [Español – América Latina](https://ai.google.dev/api/embeddings?hl=es-419)
*   [Français](https://ai.google.dev/api/embeddings?hl=fr)
*   [Indonesia](https://ai.google.dev/api/embeddings?hl=id)
*   [Italiano](https://ai.google.dev/api/embeddings?hl=it)
*   [Polski](https://ai.google.dev/api/embeddings?hl=pl)
*   [Português – Brasil](https://ai.google.dev/api/embeddings?hl=pt-br)
*   [Shqip](https://ai.google.dev/api/embeddings?hl=sq)
*   [Tiếng Việt](https://ai.google.dev/api/embeddings?hl=vi)
*   [Türkçe](https://ai.google.dev/api/embeddings?hl=tr)
*   [Русский](https://ai.google.dev/api/embeddings?hl=ru)
*   [עברית](https://ai.google.dev/api/embeddings?hl=he)
*   [العربيّة](https://ai.google.dev/api/embeddings?hl=ar)
*   [فارسی](https://ai.google.dev/api/embeddings?hl=fa)
*   [हिंदी](https://ai.google.dev/api/embeddings?hl=hi)
*   [বাংলা](https://ai.google.dev/api/embeddings?hl=bn)
*   [ภาษาไทย](https://ai.google.dev/api/embeddings?hl=th)
*   [中文 – 简体](https://ai.google.dev/api/embeddings?hl=zh-cn)
*   [中文 – 繁體](https://ai.google.dev/api/embeddings?hl=zh-tw)
*   [日本語](https://ai.google.dev/api/embeddings?hl=ja)
*   [한국어](https://ai.google.dev/api/embeddings?hl=ko)

[Sign in](https://ai.google.dev/_d/signin?continue=https%3A%2F%2Fai.google.dev%2Fapi%2Fembeddings&prompt=select_account)

[Gemini API docs](https://ai.google.dev/gemini-api/docs) [API Reference](https://ai.google.dev/api) [SDKs](https://ai.google.dev/gemini-api/docs/sdks) [Pricing](https://ai.google.dev/gemini-api/docs/pricing) [Cookbook](https://github.com/google-gemini/cookbook) More

 [![Image 2: Google AI for Developers](https://www.gstatic.com/devrel-devsite/prod/v6bfb74446ce17cd0d3af9b93bf26e056161cb79c5a6475bd6a9c25286fcb7861/googledevai/images/lockup-new.svg)](https://ai.google.dev/)

*   [Models](https://ai.google.dev/gemini-api/docs)
    
    *   More
    
    *   [Gemini API docs](https://ai.google.dev/gemini-api/docs)
    *   [API Reference](https://ai.google.dev/api)
    *   [SDKs](https://ai.google.dev/gemini-api/docs/sdks)
    *   [Pricing](https://ai.google.dev/gemini-api/docs/pricing)
    *   [Cookbook](https://github.com/google-gemini/cookbook)
*   Solutions
    *   More
*   Code assistance
    *   More
*   Showcase
    *   More
*   Community
    *   More

*   [Overview](https://ai.google.dev/api)
*   [API versions](https://ai.google.dev/gemini-api/docs/api-versions)
*   Capabilities
    
*   [Models](https://ai.google.dev/api/models)
*   [Generating content](https://ai.google.dev/api/generate-content)
*   [Tokens](https://ai.google.dev/api/tokens)
*   [Files](https://ai.google.dev/api/files)
*   [Caching](https://ai.google.dev/api/caching)
*   [Embeddings](https://ai.google.dev/api/embeddings)
*   Tuning
    
    *   [Tuning](https://ai.google.dev/api/tuning)
    *   [Permissions](https://ai.google.dev/api/tuning/permissions)
    
*   Semantic retrieval
    
    *   [Question answering](https://ai.google.dev/api/semantic-retrieval/question-answering)
    *   [Corpus](https://ai.google.dev/api/semantic-retrieval/corpora)
    *   [Document](https://ai.google.dev/api/semantic-retrieval/documents)
    *   [Chunk](https://ai.google.dev/api/semantic-retrieval/chunks)
    *   [Permissions](https://ai.google.dev/api/semantic-retrieval/permissions)
    
*   [All methods](https://ai.google.dev/api/all-methods)
*   Deprecated
    
    *   [PaLM (decomissioned)](https://ai.google.dev/api/palm)
    

*   Gemini
*   [About](https://deepmind.google/gemini)
*   [Docs](https://ai.google.dev/gemini-api/docs)
*   [API reference](https://ai.google.dev/api)
*   [Pricing](https://ai.google.dev/pricing)
*   Gemma
*   [About](https://ai.google.dev/gemma)
*   [Docs](https://ai.google.dev/gemma/docs)
*   [Gemmaverse](https://ai.google.dev/gemma/gemmaverse)

*   Build with Gemini
*   [Gemini API](https://ai.google.dev/gemini-api/docs)
*   [Google AI Studio](https://aistudio.google.com/)
*   Customize Gemma open models
*   [Gemma open models](https://ai.google.dev/gemma)
*   [Multi-framework with Keras](https://keras.io/keras_3/)
*   [Fine-tune in Colab](https://colab.sandbox.google.com/github/google/generative-ai-docs/blob/main/site/en/gemma/docs/lora_tuning.ipynb)
*   Run on-device
*   [Google AI Edge](https://ai.google.dev/edge)
*   [Gemini Nano on Android](https://developer.android.com/ai/gemini-nano)
*   [Chrome built-in web APIs](https://developer.chrome.com/docs/ai/built-in)
*   Build responsibly
*   [Responsible GenAI Toolkit](https://ai.google.dev/responsible)
*   [Secure AI Framework](https://saif.google/)

*   [Android Studio](https://developer.android.com/gemini-in-android)
*   [Chrome DevTools](https://developer.chrome.com/docs/devtools/console/understand-messages)
*   [Colab](https://colab.google/)
*   [Firebase](https://firebase.google.com/products/generative-ai)
*   [Google Cloud](https://cloud.google.com/products/gemini/code-assist)
*   [JetBrains](https://plugins.jetbrains.com/plugin/8079-google-cloud-code)
*   [Jules](https://labs.google.com/jules/home)
*   [Project IDX](https://developers.google.com/idx/guides/code-with-gemini-in-idx)
*   [VS Code](https://marketplace.visualstudio.com/items?itemName=GoogleCloudTools.cloudcode)

*   [Gemini Showcase](https://ai.google.dev/showcase)
*   [Gemini API Developer Competition](https://ai.google.dev/competition)

*   [Google AI Forum](https://discuss.ai.google.dev/)
*   [Gemini for Research](https://ai.google.dev/gemini-api/docs/gemini-for-research)

*   On this page
*   [Method: models.embedContent](https://ai.google.dev/api/embeddings#method:-models.embedcontent)
    *   [Endpoint](https://ai.google.dev/api/embeddings#endpoint)
    *   [Path parameters](https://ai.google.dev/api/embeddings#path-parameters)
    *   [Request body](https://ai.google.dev/api/embeddings#request-body)
    *   [Example request](https://ai.google.dev/api/embeddings#example-request)
    *   [Response body](https://ai.google.dev/api/embeddings#response-body)
*   [Method: models.batchEmbedContents](https://ai.google.dev/api/embeddings#method:-models.batchembedcontents)
    *   [Endpoint](https://ai.google.dev/api/embeddings#endpoint_1)
    *   [Path parameters](https://ai.google.dev/api/embeddings#path-parameters_1)
    *   [Request body](https://ai.google.dev/api/embeddings#request-body_1)
    *   [Example request](https://ai.google.dev/api/embeddings#example-request_1)
    *   [Response body](https://ai.google.dev/api/embeddings#response-body_1)
*   [EmbedContentRequest](https://ai.google.dev/api/embeddings#EmbedContentRequest)
*   [ContentEmbedding](https://ai.google.dev/api/embeddings#contentembedding)
*   [TaskType](https://ai.google.dev/api/embeddings#tasktype)

Native image generation with Gemini 2.0 Flash Experimental is now available! [Learn more](https://developers.googleblog.com/en/experiment-with-gemini-20-flash-native-image-generation/)

*   [Home](https://ai.google.dev/)
*   [Gemini API](https://ai.google.dev/gemini-api)
*   [Models](https://ai.google.dev/gemini-api/docs)
*   [API Reference](https://ai.google.dev/api)

Was this helpful?

Send feedback

Embeddings
==========

*   On this page
*   [Method: models.embedContent](https://ai.google.dev/api/embeddings#method:-models.embedcontent)
    *   [Endpoint](https://ai.google.dev/api/embeddings#endpoint)
    *   [Path parameters](https://ai.google.dev/api/embeddings#path-parameters)
    *   [Request body](https://ai.google.dev/api/embeddings#request-body)
    *   [Example request](https://ai.google.dev/api/embeddings#example-request)
    *   [Response body](https://ai.google.dev/api/embeddings#response-body)
*   [Method: models.batchEmbedContents](https://ai.google.dev/api/embeddings#method:-models.batchembedcontents)
    *   [Endpoint](https://ai.google.dev/api/embeddings#endpoint_1)
    *   [Path parameters](https://ai.google.dev/api/embeddings#path-parameters_1)
    *   [Request body](https://ai.google.dev/api/embeddings#request-body_1)
    *   [Example request](https://ai.google.dev/api/embeddings#example-request_1)
    *   [Response body](https://ai.google.dev/api/embeddings#response-body_1)
*   [EmbedContentRequest](https://ai.google.dev/api/embeddings#EmbedContentRequest)
*   [ContentEmbedding](https://ai.google.dev/api/embeddings#contentembedding)
*   [TaskType](https://ai.google.dev/api/embeddings#tasktype)

Embeddings are a numerical representation of text input that open up a number of unique use cases, such as clustering, similarity measurement and information retrieval. For an introduction, check out the [Embeddings guide](https://ai.google.dev/gemini-api/docs/embeddings).

Method: models.embedContent
---------------------------

 

*   [Endpoint](https://ai.google.dev/api/embeddings#body.HTTP_TEMPLATE)
*   [Path parameters](https://ai.google.dev/api/embeddings#body.PATH_PARAMETERS)
*   [Request body](https://ai.google.dev/api/embeddings#body.request_body)
    *   [JSON representation](https://ai.google.dev/api/embeddings#body.request_body.SCHEMA_REPRESENTATION)
*   [Response body](https://ai.google.dev/api/embeddings#body.response_body)
    *   [JSON representation](https://ai.google.dev/api/embeddings#body.EmbedContentResponse.SCHEMA_REPRESENTATION)
*   [Authorization scopes](https://ai.google.dev/api/embeddings#body.aspect)
*   [Example request](https://ai.google.dev/api/embeddings#body.codeSnippets)
    *   [Basic](https://ai.google.dev/api/embeddings#body.codeSnippets.group)

Generates a text embedding vector from the input `Content` using the specified [Gemini Embedding model](https://ai.google.dev/gemini-api/docs/models/gemini#text-embedding).

### Endpoint

post `https://generativelanguage.googleapis.com/v1beta/{model=models/*}:embedContent`  

### Path parameters

`model` `string`

Required. The model's resource name. This serves as an ID for the Model to use.

This name should match a model name returned by the `models.list` method.

Format: `models/{model}` It takes the form `models/{model}`.

### Request body

The request body contains data with the following structure:

Fields

`content` ``object (`[Content](https://ai.google.dev/api/caching#Content)`)``

Required. The content to embed. Only the `parts.text` fields will be counted.

`taskType` ``enum (`[TaskType](https://ai.google.dev/api/embeddings#v1beta.TaskType)`)``

Optional. Optional task type for which the embeddings will be used. Can only be set for `models/embedding-001`.

`title` `string`

Optional. An optional title for the text. Only applicable when TaskType is `RETRIEVAL_DOCUMENT`.

Note: Specifying a `title` for `RETRIEVAL_DOCUMENT` provides better quality embeddings for retrieval.

`outputDimensionality` `integer`

Optional. Optional reduced dimension for the output embedding. If set, excessive values in the output embedding are truncated from the end. Supported by newer models since 2024 only. You cannot set this value if using the earlier model (`models/embedding-001`).

### Example request

[Python](https://ai.google.dev/api/embeddings#python)[Node.js](https://ai.google.dev/api/embeddings#node.js)[Shell](https://ai.google.dev/api/embeddings#shell) More

```
from google import genai
from google.genai import types

client = genai.Client()
text = "Hello World!"
result = client.models.embed_content(
    model="text-embedding-004",
    contents=text,
    config=types.EmbedContentConfig(output_dimensionality=10),
)
print(result.embeddings)embed.py
```

```
// Make sure to include these imports:
// import { GoogleGenerativeAI } from "@google/generative-ai";
const genAI = new GoogleGenerativeAI(process.env.API_KEY);
const model = genAI.getGenerativeModel({
  model: "text-embedding-004",
});

const result = await model.embedContent("Hello world!");

console.log(result.embedding);embed.js
```

```
curl "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent?key=$GEMINI_API_KEY" \
-H 'Content-Type: application/json' \
-d '{"model": "models/text-embedding-004",
    "content": {
    "parts":[{
      "text": "Hello world"}]}, }' 2> /dev/null | headembed.sh
```

### Response body

The response to an `EmbedContentRequest`.

If successful, the response body contains data with the following structure:

Fields

`embedding` ``object (`[ContentEmbedding](https://ai.google.dev/api/embeddings#v1beta.ContentEmbedding)`)``

Output only. The embedding generated from the input content.

| JSON representation |
| --- |
| 
{
  "embedding": {
    object (`[ContentEmbedding](https://ai.google.dev/api/embeddings#v1beta.ContentEmbedding)`)
  }
}

 |

Method: models.batchEmbedContents
---------------------------------

 

*   [Endpoint](https://ai.google.dev/api/embeddings#body.HTTP_TEMPLATE)
*   [Path parameters](https://ai.google.dev/api/embeddings#body.PATH_PARAMETERS)
*   [Request body](https://ai.google.dev/api/embeddings#body.request_body)
    *   [JSON representation](https://ai.google.dev/api/embeddings#body.request_body.SCHEMA_REPRESENTATION)
*   [Response body](https://ai.google.dev/api/embeddings#body.response_body)
    *   [JSON representation](https://ai.google.dev/api/embeddings#body.BatchEmbedContentsResponse.SCHEMA_REPRESENTATION)
*   [Authorization scopes](https://ai.google.dev/api/embeddings#body.aspect)
*   [Example request](https://ai.google.dev/api/embeddings#body.codeSnippets)
    *   [Basic](https://ai.google.dev/api/embeddings#body.codeSnippets.group)
*   [EmbedContentRequest](https://ai.google.dev/api/embeddings#EmbedContentRequest)
    *   [JSON representation](https://ai.google.dev/api/embeddings#EmbedContentRequest.SCHEMA_REPRESENTATION)

Generates multiple embedding vectors from the input `Content` which consists of a batch of strings represented as `EmbedContentRequest` objects.

### Endpoint

post `https://generativelanguage.googleapis.com/v1beta/{model=models/*}:batchEmbedContents`  

### Path parameters

`model` `string`

Required. The model's resource name. This serves as an ID for the Model to use.

This name should match a model name returned by the `models.list` method.

Format: `models/{model}` It takes the form `models/{model}`.

### Request body

The request body contains data with the following structure:

Fields

`requests[]` ``object (`[EmbedContentRequest](https://ai.google.dev/api/embeddings#EmbedContentRequest)`)``

Required. Embed requests for the batch. The model in each of these requests must match the model specified `BatchEmbedContentsRequest.model`.

### Example request

[Python](https://ai.google.dev/api/embeddings#python)[Node.js](https://ai.google.dev/api/embeddings#node.js)[Shell](https://ai.google.dev/api/embeddings#shell) More

```
from google import genai
from google.genai import types

client = genai.Client()
texts = [
    "What is the meaning of life?",
    "How much wood would a woodchuck chuck?",
    "How does the brain work?",
]
result = client.models.embed_content(
    model="text-embedding-004",
    contents=texts,
    config=types.EmbedContentConfig(output_dimensionality=10),
)
print(result.embeddings)embed.py
```

```
// Make sure to include these imports:
// import { GoogleGenerativeAI } from "@google/generative-ai";
const genAI = new GoogleGenerativeAI(process.env.API_KEY);
const model = genAI.getGenerativeModel({
  model: "text-embedding-004",
});

function textToRequest(text) {
  return { content: { role: "user", parts: [{ text }] } };
}

const result = await model.batchEmbedContents({
  requests: [
    textToRequest("What is the meaning of life?"),
    textToRequest("How much wood would a woodchuck chuck?"),
    textToRequest("How does the brain work?"),
  ],
});

console.log(result.embeddings);embed.js
```

```
curl "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:batchEmbedContents?key=$GEMINI_API_KEY" \
-H 'Content-Type: application/json' \
-d '{"requests": [{
      "model": "models/text-embedding-004",
      "content": {
      "parts":[{
        "text": "What is the meaning of life?"}]}, },
      {
      "model": "models/text-embedding-004",
      "content": {
      "parts":[{
        "text": "How much wood would a woodchuck chuck?"}]}, },
      {
      "model": "models/text-embedding-004",
      "content": {
      "parts":[{
        "text": "How does the brain work?"}]}, }, ]}' 2> /dev/null | grep -C 5 valuesembed.sh
```

### Response body

The response to a `BatchEmbedContentsRequest`.

If successful, the response body contains data with the following structure:

Fields

`embeddings[]` ``object (`[ContentEmbedding](https://ai.google.dev/api/embeddings#v1beta.ContentEmbedding)`)``

Output only. The embeddings for each request, in the same order as provided in the batch request.

| JSON representation |
| --- |
| 
{
  "embeddings": \[
    {
      object (`[ContentEmbedding](https://ai.google.dev/api/embeddings#v1beta.ContentEmbedding)`)
    }
  \]
}

 |

EmbedContentRequest
-------------------

Request containing the `Content` for the model to embed.

Fields

`model` `string`

Required. The model's resource name. This serves as an ID for the Model to use.

This name should match a model name returned by the `models.list` method.

Format: `models/{model}`

`content` ``object (`[Content](https://ai.google.dev/api/caching#Content)`)``

Required. The content to embed. Only the `parts.text` fields will be counted.

`taskType` ``enum (`[TaskType](https://ai.google.dev/api/embeddings#v1beta.TaskType)`)``

Optional. Optional task type for which the embeddings will be used. Can only be set for `models/embedding-001`.

`title` `string`

Optional. An optional title for the text. Only applicable when TaskType is `RETRIEVAL_DOCUMENT`.

Note: Specifying a `title` for `RETRIEVAL_DOCUMENT` provides better quality embeddings for retrieval.

`outputDimensionality` `integer`

Optional. Optional reduced dimension for the output embedding. If set, excessive values in the output embedding are truncated from the end. Supported by newer models since 2024 only. You cannot set this value if using the earlier model (`models/embedding-001`).

| JSON representation |
| --- |
| 
{
  "model": string,
  "content": {
    object (`[Content](https://ai.google.dev/api/caching#Content)`)
  },
  "taskType": enum (`[TaskType](https://ai.google.dev/api/embeddings#v1beta.TaskType)`),
  "title": string,
  "outputDimensionality": integer
}

 |

ContentEmbedding
----------------

 

*   [JSON representation](https://ai.google.dev/api/embeddings#SCHEMA_REPRESENTATION)

A list of floats representing an embedding.

Fields

`values[]` `number`

The embedding values.

| JSON representation |
| --- |
| 
{
  "values": \[
    number
  \]
}

 |

TaskType
--------

 

Type of task for which the embedding will be used.

 
| Enums |
| --- |
| `TASK_TYPE_UNSPECIFIED` | Unset value, which will default to one of the other enum values. |
| `RETRIEVAL_QUERY` | Specifies the given text is a query in a search/retrieval setting. |
| `RETRIEVAL_DOCUMENT` | Specifies the given text is a document from the corpus being searched. |
| `SEMANTIC_SIMILARITY` | Specifies the given text will be used for STS. |
| `CLASSIFICATION` | Specifies that the given text will be classified. |
| `CLUSTERING` | Specifies that the embeddings will be used for clustering. |
| `QUESTION_ANSWERING` | Specifies that the given text will be used for question answering. |
| `FACT_VERIFICATION` | Specifies that the given text will be used for fact verification. |

Was this helpful?

Send feedback

Except as otherwise noted, the content of this page is licensed under the [Creative Commons Attribution 4.0 License](https://creativecommons.org/licenses/by/4.0/), and code samples are licensed under the [Apache 2.0 License](https://www.apache.org/licenses/LICENSE-2.0). For details, see the [Google Developers Site Policies](https://developers.google.com/site-policies). Java is a registered trademark of Oracle and/or its affiliates.

Last updated 2025-03-06 UTC.

Need to tell us more? \[\[\["Easy to understand","easyToUnderstand","thumb-up"\],\["Solved my problem","solvedMyProblem","thumb-up"\],\["Other","otherUp","thumb-up"\]\],\[\["Missing the information I need","missingTheInformationINeed","thumb-down"\],\["Too complicated / too many steps","tooComplicatedTooManySteps","thumb-down"\],\["Out of date","outOfDate","thumb-down"\],\["Samples / code issue","samplesCodeIssue","thumb-down"\],\["Other","otherDown","thumb-down"\]\],\["Last updated 2025-03-06 UTC."\],\[\],\[\]\]

*   [Terms](https://policies.google.com/terms)
*   [Privacy](https://policies.google.com/privacy)
*   [Manage cookies](https://ai.google.dev/api/embeddings#)

*   [English](https://ai.google.dev/api/embeddings)
*   [Deutsch](https://ai.google.dev/api/embeddings?hl=de)
*   [Español – América Latina](https://ai.google.dev/api/embeddings?hl=es-419)
*   [Français](https://ai.google.dev/api/embeddings?hl=fr)
*   [Indonesia](https://ai.google.dev/api/embeddings?hl=id)
*   [Italiano](https://ai.google.dev/api/embeddings?hl=it)
*   [Polski](https://ai.google.dev/api/embeddings?hl=pl)
*   [Português – Brasil](https://ai.google.dev/api/embeddings?hl=pt-br)
*   [Shqip](https://ai.google.dev/api/embeddings?hl=sq)
*   [Tiếng Việt](https://ai.google.dev/api/embeddings?hl=vi)
*   [Türkçe](https://ai.google.dev/api/embeddings?hl=tr)
*   [Русский](https://ai.google.dev/api/embeddings?hl=ru)
*   [עברית](https://ai.google.dev/api/embeddings?hl=he)
*   [العربيّة](https://ai.google.dev/api/embeddings?hl=ar)
*   [فارسی](https://ai.google.dev/api/embeddings?hl=fa)
*   [हिंदी](https://ai.google.dev/api/embeddings?hl=hi)
*   [বাংলা](https://ai.google.dev/api/embeddings?hl=bn)
*   [ภาษาไทย](https://ai.google.dev/api/embeddings?hl=th)
*   [中文 – 简体](https://ai.google.dev/api/embeddings?hl=zh-cn)
*   [中文 – 繁體](https://ai.google.dev/api/embeddings?hl=zh-tw)
*   [日本語](https://ai.google.dev/api/embeddings?hl=ja)
*   [한국어](https://ai.google.dev/api/embeddings?hl=ko)

