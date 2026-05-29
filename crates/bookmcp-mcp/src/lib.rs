#![forbid(unsafe_code)]

use std::{
    future,
    path::{Path, PathBuf},
    sync::Arc,
};

use bookmcp_core::{
    BookId, BookMcpError, BookMetadata, Chapter, Chunk, ChunkId, Citation, MAX_TOP_K, PageNumber,
    Result as CoreResult, SearchQuery, SearchResult,
};
use bookmcp_index::{IndexManager, SearchService};
use bookmcp_store::BookStore;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, GetPromptRequestParams, GetPromptResult, ListPromptsResult,
        ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams, Prompt,
        PromptArgument, PromptMessage, PromptMessageRole, RawResource, RawResourceTemplate,
        ReadResourceRequestParams, ReadResourceResult, Resource, ResourceContents,
        ResourceTemplate, ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_CHARS: usize = 8_000;
const MAX_MAX_CHARS: usize = 20_000;
const DEFAULT_CONTEXT_WINDOW: usize = 1;
const MAX_CONTEXT_WINDOW: usize = 5;
const MAX_LIST_LIMIT: usize = 100;

/// Read-only BookMCP MCP server and direct service facade.
#[derive(Clone)]
pub struct BookMcpServer {
    data_dir: Arc<PathBuf>,
    tool_router: ToolRouter<Self>,
}

impl BookMcpServer {
    /// Create a server rooted at a BookMCP data directory.
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            data_dir: Arc::new(data_dir.as_ref().to_path_buf()),
            tool_router: Self::tool_router(),
        }
    }

    /// Return all tool names in deterministic order.
    pub fn tool_names(&self) -> Vec<&'static str> {
        vec![
            "book_find_definitions",
            "book_find_examples",
            "book_get_chunk",
            "book_get_context",
            "book_get_metadata",
            "book_get_page",
            "book_get_toc",
            "book_list_books",
            "book_search",
        ]
    }

    /// List books with optional pagination.
    pub fn book_list_books(&self, input: BookListBooksInput) -> CoreResult<BookListBooksOutput> {
        let store = self.store()?;
        let offset = input.offset.unwrap_or(0);
        let limit = input.limit.unwrap_or(MAX_LIST_LIMIT).min(MAX_LIST_LIMIT);
        let books = store
            .list_books()?
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(BookSummary::from)
            .collect();

        Ok(BookListBooksOutput { books })
    }

    /// Return detailed metadata for one book.
    pub fn book_get_metadata(
        &self,
        input: BookGetMetadataInput,
    ) -> CoreResult<BookGetMetadataOutput> {
        let metadata = self.store()?.get_book(&input.book_id)?;
        Ok(BookGetMetadataOutput {
            source_sha256: metadata.source_sha256.clone(),
            page_count: metadata.page_count,
            chapter_count: metadata.chapter_count,
            chunk_count: metadata.chunk_count,
            metadata,
        })
    }

    /// Return detected table-of-contents information.
    pub fn book_get_toc(&self, input: BookGetTocInput) -> CoreResult<BookGetTocOutput> {
        let chapters = self.store()?.list_chapters(&input.book_id)?;
        let message = if chapters.is_empty() {
            Some("No chapters detected; use page resources for this book.".to_owned())
        } else {
            None
        };
        Ok(BookGetTocOutput {
            book_id: input.book_id,
            chapters,
            message,
        })
    }

    /// Run keyword search.
    pub fn book_search(&self, input: BookSearchInput) -> CoreResult<BookSearchOutput> {
        if !matches!(
            input.mode.unwrap_or(SearchModeInput::Keyword),
            SearchModeInput::Keyword
        ) {
            return Err(BookMcpError::InvalidLimit { value: 0, max: 0 });
        }

        let output = self.search_service()?.search(SearchQuery {
            query: input.query,
            book_id: input.book_id,
            top_k: input.top_k.map(|top_k| top_k.min(MAX_TOP_K)),
        })?;

        Ok(BookSearchOutput {
            results: output.results,
            message: output.message,
        })
    }

    /// Return extracted text for one page.
    pub fn book_get_page(&self, input: BookGetPageInput) -> CoreResult<BookGetPageOutput> {
        let page = self.store()?.get_page(&input.book_id, input.page_number)?;
        let (text, truncated) = cap_text(&page.text, input.max_chars);
        Ok(BookGetPageOutput {
            book_id: page.book_id,
            page_number: page.page_number,
            text,
            citation: page.citation,
            truncated,
        })
    }

    /// Return one chunk and optional neighbor IDs.
    pub fn book_get_chunk(&self, input: BookGetChunkInput) -> CoreResult<BookGetChunkOutput> {
        let store = self.store()?;
        let chunk = store.get_chunk(&input.book_id, &input.chunk_id)?;
        let (previous_chunk_id, next_chunk_id) = if input.include_neighbors.unwrap_or(false) {
            let neighbors = store.get_chunks_around(&input.book_id, &input.chunk_id, 1, 1)?;
            neighbor_ids(&neighbors, &input.chunk_id)
        } else {
            (None, None)
        };

        Ok(BookGetChunkOutput {
            chunk,
            previous_chunk_id,
            next_chunk_id,
        })
    }

    /// Return capped surrounding context for a chunk.
    pub fn book_get_context(&self, input: BookGetContextInput) -> CoreResult<BookGetContextOutput> {
        let before = input
            .before
            .unwrap_or(DEFAULT_CONTEXT_WINDOW)
            .min(MAX_CONTEXT_WINDOW);
        let after = input
            .after
            .unwrap_or(DEFAULT_CONTEXT_WINDOW)
            .min(MAX_CONTEXT_WINDOW);
        let max_chars = capped_max_chars(input.max_chars);
        let chunks =
            self.store()?
                .get_chunks_around(&input.book_id, &input.chunk_id, before, after)?;
        let (chunks, total_chars, truncated) = cap_context_chunks(chunks, max_chars);

        Ok(BookGetContextOutput {
            chunks,
            total_chars,
            truncated,
        })
    }

    /// Find likely definitions for a term.
    pub fn book_find_definitions(
        &self,
        input: BookFindDefinitionsInput,
    ) -> CoreResult<BookSearchOutput> {
        let mut output = self.book_search(BookSearchInput {
            query: input.term,
            book_id: input.book_id,
            top_k: input.top_k,
            mode: Some(SearchModeInput::Keyword),
        })?;
        rank_pattern_matches(
            &mut output.results,
            &["definition", "defined", "means", "is a"],
        );
        Ok(output)
    }

    /// Find likely examples for a topic.
    pub fn book_find_examples(&self, input: BookFindExamplesInput) -> CoreResult<BookSearchOutput> {
        let mut output = self.book_search(BookSearchInput {
            query: input.topic,
            book_id: input.book_id,
            top_k: input.top_k,
            mode: Some(SearchModeInput::Keyword),
        })?;
        rank_pattern_matches(&mut output.results, &["example", "for instance", "such as"]);
        Ok(output)
    }

    /// Return resource templates advertised by the MCP server.
    pub fn resource_templates(&self) -> Vec<RawResourceTemplate> {
        [
            ("book://{book_id}/metadata", "Book metadata"),
            ("book://{book_id}/toc", "Book table of contents"),
            ("book://{book_id}/page/{page_number}", "Book page"),
            ("book://{book_id}/chunk/{chunk_id}", "Book chunk"),
            ("book://{book_id}/chapter/{chapter_id}", "Book chapter"),
        ]
        .into_iter()
        .map(|(uri_template, name)| {
            let mut template = RawResourceTemplate::new(uri_template, name);
            template.description = Some("Read-only BookMCP resource".to_owned());
            template.mime_type = Some("text/plain".to_owned());
            template
        })
        .collect()
    }

    /// Read a `book://` resource without touching arbitrary file paths.
    pub fn read_book_resource(&self, uri: &str) -> CoreResult<ResourceReadOutput> {
        let parsed = parse_book_uri(uri)?;
        let text = match parsed.kind {
            ResourceKind::Metadata => {
                serde_json::to_string_pretty(&self.book_get_metadata(BookGetMetadataInput {
                    book_id: parsed.book_id,
                })?)
                .map_err(|error| BookMcpError::Mcp(error.to_string()))?
            }
            ResourceKind::Toc => {
                serde_json::to_string_pretty(&self.book_get_toc(BookGetTocInput {
                    book_id: parsed.book_id,
                })?)
                .map_err(|error| BookMcpError::Mcp(error.to_string()))?
            }
            ResourceKind::Page(page_number) => {
                let page = self.book_get_page(BookGetPageInput {
                    book_id: parsed.book_id,
                    page_number,
                    max_chars: Some(MAX_MAX_CHARS),
                })?;
                format!("{}\n{}", page.citation.format(), page.text)
            }
            ResourceKind::Chunk(chunk_id) => {
                let chunk = self.book_get_chunk(BookGetChunkInput {
                    book_id: parsed.book_id,
                    chunk_id,
                    include_neighbors: Some(false),
                })?;
                format!("{}\n{}", chunk.chunk.citation.format(), chunk.chunk.text)
            }
            ResourceKind::Chapter(chapter_id) => {
                self.chapter_resource_text(&parsed.book_id, &chapter_id)?
            }
        };

        Ok(ResourceReadOutput {
            uri: uri.to_owned(),
            text,
            mime_type: "text/plain".to_owned(),
        })
    }

    /// Return the prompt catalog in deterministic order.
    pub fn prompt_catalog(&self) -> Vec<Prompt> {
        prompt_specs()
            .iter()
            .map(|spec| {
                Prompt::new(
                    spec.name,
                    Some(spec.description),
                    Some(prompt_arguments(spec)),
                )
                .with_title(spec.title)
            })
            .collect()
    }

    /// Return one prompt body.
    pub fn get_prompt_text(&self, name: &str) -> CoreResult<String> {
        prompt_specs()
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| spec.body.to_owned())
            .ok_or_else(|| BookMcpError::Mcp(format!("unknown prompt `{name}`")))
    }

    fn store(&self) -> CoreResult<BookStore> {
        BookStore::open(self.data_dir.as_ref())
    }

    fn search_service(&self) -> CoreResult<SearchService> {
        let index = IndexManager::create_or_open(self.data_dir.join("index"))?;
        Ok(SearchService::new(index))
    }

    fn chapter_resource_text(
        &self,
        book_id: &BookId,
        chapter_id: &bookmcp_core::ChapterId,
    ) -> CoreResult<String> {
        let store = self.store()?;
        let chapter = store.get_chapter(book_id, chapter_id)?;
        let mut text = format!(
            "{}\nPages {}-{}\n",
            chapter.title, chapter.page_start, chapter.page_end
        );
        for page in store.list_pages(book_id)? {
            if page.page_number >= chapter.page_start && page.page_number <= chapter.page_end {
                text.push_str(&page.citation.format());
                text.push('\n');
                text.push_str(&page.text);
                text.push('\n');
            }
        }
        Ok(text)
    }

    fn to_mcp_json<T: Serialize>(&self, value: T) -> std::result::Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::json(value)?]))
    }
}

#[tool_router(router = tool_router)]
impl BookMcpServer {
    #[tool(name = "book_list_books", description = "List ingested books")]
    fn book_list_books_tool(
        &self,
        Parameters(input): Parameters<BookListBooksInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_list_books");
        self.to_mcp_json(self.book_list_books(input).map_err(to_mcp_error)?)
    }

    #[tool(name = "book_get_metadata", description = "Get book metadata")]
    fn book_get_metadata_tool(
        &self,
        Parameters(input): Parameters<BookGetMetadataInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_get_metadata", book_id = %input.book_id);
        self.to_mcp_json(self.book_get_metadata(input).map_err(to_mcp_error)?)
    }

    #[tool(
        name = "book_get_toc",
        description = "Get a detected table of contents"
    )]
    fn book_get_toc_tool(
        &self,
        Parameters(input): Parameters<BookGetTocInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_get_toc", book_id = %input.book_id);
        self.to_mcp_json(self.book_get_toc(input).map_err(to_mcp_error)?)
    }

    #[tool(
        name = "book_search",
        description = "Keyword search across ingested books"
    )]
    fn book_search_tool(
        &self,
        Parameters(input): Parameters<BookSearchInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_search", book_id = ?input.book_id, top_k = ?input.top_k);
        self.to_mcp_json(self.book_search(input).map_err(to_mcp_error)?)
    }

    #[tool(
        name = "book_get_page",
        description = "Fetch extracted text for one page"
    )]
    fn book_get_page_tool(
        &self,
        Parameters(input): Parameters<BookGetPageInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_get_page", book_id = %input.book_id, page = %input.page_number);
        self.to_mcp_json(self.book_get_page(input).map_err(to_mcp_error)?)
    }

    #[tool(name = "book_get_chunk", description = "Fetch one citable chunk")]
    fn book_get_chunk_tool(
        &self,
        Parameters(input): Parameters<BookGetChunkInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_get_chunk", book_id = %input.book_id, chunk_id = %input.chunk_id);
        self.to_mcp_json(self.book_get_chunk(input).map_err(to_mcp_error)?)
    }

    #[tool(
        name = "book_get_context",
        description = "Fetch capped surrounding chunk context"
    )]
    fn book_get_context_tool(
        &self,
        Parameters(input): Parameters<BookGetContextInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_get_context", book_id = %input.book_id, chunk_id = %input.chunk_id);
        self.to_mcp_json(self.book_get_context(input).map_err(to_mcp_error)?)
    }

    #[tool(
        name = "book_find_definitions",
        description = "Find likely definition passages"
    )]
    fn book_find_definitions_tool(
        &self,
        Parameters(input): Parameters<BookFindDefinitionsInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_find_definitions", book_id = ?input.book_id);
        self.to_mcp_json(self.book_find_definitions(input).map_err(to_mcp_error)?)
    }

    #[tool(
        name = "book_find_examples",
        description = "Find likely example passages"
    )]
    fn book_find_examples_tool(
        &self,
        Parameters(input): Parameters<BookFindExamplesInput>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tracing::info!(tool = "book_find_examples", book_id = ?input.book_id);
        self.to_mcp_json(self.book_find_examples(input).map_err(to_mcp_error)?)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BookMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_instructions(
            "Read-only local BookMCP server. Search before fetching chunks and answer with citations.",
        )
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListResourcesResult, McpError>> + Send + '_ {
        let result = self
            .list_resource_links()
            .map(ListResourcesResult::with_all_items)
            .map_err(to_mcp_error);
        future::ready(result)
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListResourceTemplatesResult, McpError>> + Send + '_
    {
        let templates = self
            .resource_templates()
            .into_iter()
            .map(rmcp::model::AnnotateAble::no_annotation)
            .collect::<Vec<ResourceTemplate>>();
        future::ready(Ok(ListResourceTemplatesResult::with_all_items(templates)))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ReadResourceResult, McpError>> + Send + '_ {
        let uri = request.uri;
        let result = self
            .read_book_resource(&uri)
            .map(|resource| {
                ReadResourceResult::new(vec![
                    ResourceContents::text(resource.text, resource.uri)
                        .with_mime_type(resource.mime_type),
                ])
            })
            .map_err(to_mcp_error);
        future::ready(result)
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListPromptsResult, McpError>> + Send + '_ {
        future::ready(Ok(ListPromptsResult::with_all_items(self.prompt_catalog())))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<GetPromptResult, McpError>> + Send + '_ {
        let result = self
            .get_prompt_text(&request.name)
            .map(|body| {
                GetPromptResult::new(vec![PromptMessage::new_text(PromptMessageRole::User, body)])
                    .with_description(format!("BookMCP prompt {}", request.name))
            })
            .map_err(to_mcp_error);
        future::ready(result)
    }
}

/// Serve BookMCP over MCP stdio.
pub async fn serve_stdio(data_dir: impl AsRef<Path>) -> CoreResult<()> {
    let running = BookMcpServer::new(data_dir)
        .serve(stdio())
        .await
        .map_err(|error| BookMcpError::Mcp(error.to_string()))?;
    running
        .waiting()
        .await
        .map(|_| ())
        .map_err(|error| BookMcpError::Mcp(error.to_string()))
}

impl BookMcpServer {
    fn list_resource_links(&self) -> CoreResult<Vec<Resource>> {
        use rmcp::model::AnnotateAble;

        let mut resources = Vec::new();
        for book in self.store()?.list_books()? {
            resources.push(
                RawResource::new(
                    format!("book://{}/metadata", book.book_id),
                    format!("{} metadata", book.title),
                )
                .no_annotation(),
            );
            resources.push(
                RawResource::new(
                    format!("book://{}/toc", book.book_id),
                    format!("{} table of contents", book.title),
                )
                .no_annotation(),
            );
        }
        Ok(resources)
    }
}

/// Input for `book_list_books`.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookListBooksInput {
    /// Optional offset.
    pub offset: Option<usize>,
    /// Optional limit, capped server-side.
    pub limit: Option<usize>,
}

/// Book summary returned by `book_list_books`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookSummary {
    pub book_id: BookId,
    pub title: String,
    pub author: Option<String>,
    pub page_count: u32,
    pub chunk_count: u32,
    pub ingested_at: String,
}

impl From<BookMetadata> for BookSummary {
    fn from(metadata: BookMetadata) -> Self {
        Self {
            book_id: metadata.book_id,
            title: metadata.title,
            author: metadata.author,
            page_count: metadata.page_count,
            chunk_count: metadata.chunk_count,
            ingested_at: metadata.ingested_at,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookListBooksOutput {
    pub books: Vec<BookSummary>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetMetadataInput {
    pub book_id: BookId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetMetadataOutput {
    pub metadata: BookMetadata,
    pub source_sha256: String,
    pub page_count: u32,
    pub chapter_count: u32,
    pub chunk_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetTocInput {
    pub book_id: BookId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetTocOutput {
    pub book_id: BookId,
    pub chapters: Vec<Chapter>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchModeInput {
    Keyword,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookSearchInput {
    pub query: String,
    pub book_id: Option<BookId>,
    pub top_k: Option<usize>,
    pub mode: Option<SearchModeInput>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct BookSearchOutput {
    pub results: Vec<SearchResult>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetPageInput {
    pub book_id: BookId,
    pub page_number: PageNumber,
    pub max_chars: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetPageOutput {
    pub book_id: BookId,
    pub page_number: PageNumber,
    pub text: String,
    pub citation: Citation,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetChunkInput {
    pub book_id: BookId,
    pub chunk_id: ChunkId,
    pub include_neighbors: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetChunkOutput {
    pub chunk: Chunk,
    pub previous_chunk_id: Option<ChunkId>,
    pub next_chunk_id: Option<ChunkId>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetContextInput {
    pub book_id: BookId,
    pub chunk_id: ChunkId,
    pub before: Option<usize>,
    pub after: Option<usize>,
    pub max_chars: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ContextChunk {
    pub chunk_id: ChunkId,
    pub page_start: PageNumber,
    pub page_end: PageNumber,
    pub text: String,
    pub citation: Citation,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetContextOutput {
    pub chunks: Vec<ContextChunk>,
    pub total_chars: usize,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookFindDefinitionsInput {
    pub book_id: Option<BookId>,
    pub term: String,
    pub top_k: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookFindExamplesInput {
    pub book_id: Option<BookId>,
    pub topic: String,
    pub top_k: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ResourceReadOutput {
    pub uri: String,
    pub text: String,
    pub mime_type: String,
}

fn cap_text(text: &str, max_chars: Option<usize>) -> (String, bool) {
    let max_chars = capped_max_chars(max_chars);
    if text.chars().count() <= max_chars {
        (text.to_owned(), false)
    } else {
        (text.chars().take(max_chars).collect(), true)
    }
}

fn capped_max_chars(max_chars: Option<usize>) -> usize {
    max_chars.unwrap_or(DEFAULT_MAX_CHARS).min(MAX_MAX_CHARS)
}

fn cap_context_chunks(chunks: Vec<Chunk>, max_chars: usize) -> (Vec<ContextChunk>, usize, bool) {
    let mut remaining = max_chars;
    let mut total_chars = 0;
    let mut truncated = false;
    let mut output = Vec::new();

    for chunk in chunks {
        let chunk_len = chunk.text.chars().count();
        let text = if chunk_len <= remaining {
            remaining = remaining.saturating_sub(chunk_len);
            chunk.text.clone()
        } else {
            truncated = true;
            let capped = chunk.text.chars().take(remaining).collect::<String>();
            remaining = 0;
            capped
        };
        total_chars += text.chars().count();
        output.push(ContextChunk {
            chunk_id: chunk.chunk_id,
            page_start: chunk.page_start,
            page_end: chunk.page_end,
            text,
            citation: chunk.citation,
        });
        if remaining == 0 {
            truncated = truncated || !output.is_empty();
            break;
        }
    }

    (output, total_chars, truncated)
}

fn neighbor_ids(chunks: &[Chunk], target: &ChunkId) -> (Option<ChunkId>, Option<ChunkId>) {
    let mut previous = None;
    let mut next = None;

    for (index, chunk) in chunks.iter().enumerate() {
        if &chunk.chunk_id == target {
            previous = index
                .checked_sub(1)
                .and_then(|previous_index| chunks.get(previous_index))
                .map(|chunk| chunk.chunk_id.clone());
            next = chunks.get(index + 1).map(|chunk| chunk.chunk_id.clone());
            break;
        }
    }

    (previous, next)
}

fn rank_pattern_matches(results: &mut [SearchResult], patterns: &[&str]) {
    results.sort_by_key(|result| {
        let haystack = result.snippet.to_ascii_lowercase();
        !patterns.iter().any(|pattern| haystack.contains(pattern))
    });
}

struct ParsedBookUri {
    book_id: BookId,
    kind: ResourceKind,
}

enum ResourceKind {
    Metadata,
    Toc,
    Page(PageNumber),
    Chunk(ChunkId),
    Chapter(bookmcp_core::ChapterId),
}

fn parse_book_uri(uri: &str) -> CoreResult<ParsedBookUri> {
    let Some(rest) = uri.strip_prefix("book://") else {
        return Err(BookMcpError::Mcp(format!(
            "unsupported resource URI `{uri}`"
        )));
    };
    let parts = rest.split('/').collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(BookMcpError::Mcp(format!("invalid resource URI `{uri}`")));
    }
    let book_id = BookId::parse(parts[0].to_owned())?;
    let kind = match parts.as_slice() {
        [_, "metadata"] => ResourceKind::Metadata,
        [_, "toc"] => ResourceKind::Toc,
        [_, "page", page] => ResourceKind::Page(
            page.parse::<u32>()
                .map_err(|_| BookMcpError::Mcp(format!("invalid page resource URI `{uri}`")))
                .and_then(PageNumber::new)?,
        ),
        [_, "chunk", chunk_id] => ResourceKind::Chunk(ChunkId::parse((*chunk_id).to_owned())?),
        [_, "chapter", chapter_id] => {
            ResourceKind::Chapter(bookmcp_core::ChapterId::parse((*chapter_id).to_owned())?)
        }
        _ => return Err(BookMcpError::Mcp(format!("invalid resource URI `{uri}`"))),
    };

    Ok(ParsedBookUri { book_id, kind })
}

struct PromptSpec {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    body: &'static str,
    arguments: &'static [(&'static str, bool, &'static str)],
}

fn prompt_specs() -> &'static [PromptSpec] {
    &[
        PromptSpec {
            name: "ask_book_with_citations",
            title: "Ask Book With Citations",
            description: "Ask a question and require search/chunk evidence before answering.",
            body: "Use book_search first, then book_get_chunk or book_get_context before answering. Answer only from retrieved book evidence and include citations.",
            arguments: &[
                ("question", true, "Question to answer"),
                ("book_id", false, "Optional book ID"),
            ],
        },
        PromptSpec {
            name: "compare_book_sections",
            title: "Compare Book Sections",
            description: "Compare two chapters or sections with citations.",
            body: "Use book_get_context, book_get_chunk, or chapter resources for both sections. Compare claims, agreements, and tensions with citations.",
            arguments: &[
                ("first_section", true, "First section"),
                ("second_section", true, "Second section"),
            ],
        },
        PromptSpec {
            name: "extract_actionable_rules",
            title: "Extract Actionable Rules",
            description: "Extract principles, rules, or checklists from a book section.",
            body: "Use book_search and book_get_context to retrieve the section. Extract actionable rules as a checklist and attach citations to each rule.",
            arguments: &[("section", true, "Section, chunk, or topic")],
        },
        PromptSpec {
            name: "review_against_book",
            title: "Review Against Book",
            description: "Review user-provided text or code against principles from a chosen book.",
            body: "Use book_search to find relevant book principles, then review the supplied text against those passages. Include citations for every book-derived critique.",
            arguments: &[
                ("book_id", true, "Book ID"),
                ("subject", true, "Text or code to review"),
            ],
        },
        PromptSpec {
            name: "study_chapter",
            title: "Study Chapter",
            description: "Turn a chapter into a cited study guide.",
            body: "Use book_get_toc to identify the chapter, then fetch chapter/page/chunk context. Produce a study guide with summary, key terms, questions, and citations.",
            arguments: &[
                ("book_id", true, "Book ID"),
                ("chapter_id", true, "Chapter ID"),
            ],
        },
    ]
}

fn prompt_arguments(spec: &PromptSpec) -> Vec<PromptArgument> {
    spec.arguments
        .iter()
        .map(|(name, required, description)| {
            PromptArgument::new(*name)
                .with_description(*description)
                .with_required(*required)
        })
        .collect()
}

fn to_mcp_error(error: BookMcpError) -> McpError {
    McpError::internal_error(error.to_string(), None)
}
