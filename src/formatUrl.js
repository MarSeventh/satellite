export function formatUrl(url, filename, format) {
  switch (format) {
    case "markdown":
      return `![${filename}](${url})`;
    case "html":
      return `<img src="${url}" alt="${filename}" />`;
    case "bbcode":
      return `[img]${url}[/img]`;
    case "raw":
    default:
      return url;
  }
}

export const FORMAT_OPTIONS = [
  { value: "raw", label: "原始链接" },
  { value: "markdown", label: "Markdown" },
  { value: "html", label: "HTML" },
  { value: "bbcode", label: "BBCode" },
];
