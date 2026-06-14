import { useState, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

// TypeScript interface matching the Rust BookInfo struct
interface BookInfo {
  path: string;
  file_type: string;
  title: string | null;
  author: string | null;
  size_bytes: number;
  content_preview: string;
}

function App() {
  const [books, setBooks] = useState<BookInfo[]>([]);
  const [query, setQuery] = useState<string>('');
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [status, setStatus] = useState<string>('');
  const [hasIndexed, setHasIndexed] = useState<boolean>(false);

  useEffect(() => {
    if (!hasIndexed) return;
    const timer = setTimeout(async () => {
      try {
        // Type the invoke response as an array of BookInfo
        const results = await invoke<BookInfo[]>('search_books', { query });
        setBooks(results);
      } catch (err) {
        console.error('Search failed:', err);
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [query, hasIndexed]);

  const handleSelectDirectory = async () => {
    const dir = await open({ directory: true, multiple: false, title: 'Select directory to index' });
    if (!dir) return;

    setIsLoading(true);
    setStatus('Extracting text and indexing...');
    setHasIndexed(false);

    try {
      // Type the invoke response as a number
      const count = await invoke<number>('index_directory', { dirPath: dir });
      setHasIndexed(true);
      setStatus(`✅ Indexed ${count} books. Search ready.`);
      setBooks([]);
    } catch (err) {
      setStatus(`❌ Error: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  const formatSize = (bytes: number): string => (bytes / 1024 / 1024).toFixed(2) + ' MB';

  const highlightText = (text: string | null, term: string) => {
    if (!term || !text) return text || '';
    const parts = text.split(new RegExp(`(${term})`, 'gi'));
    return parts.map((part, i) => 
      part.toLowerCase() === term.toLowerCase() 
        ? <mark key={i}>{part}</mark> 
        : part
    );
  };

  return (
    <div className="container">
      <header>
        <h1>📚 EPUB & PDF Content Indexer</h1>
        <button onClick={handleSelectDirectory} disabled={isLoading}>
          {isLoading ? 'Indexing...' : 'Select Directory'}
        </button>
      </header>

      <div className="search-bar">
        <input
          type="text"
          placeholder="Search by title, author, or content..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          disabled={!hasIndexed}
        />
      </div>

      {status && <p className={`status ${status.startsWith('❌') ? 'error' : 'success'}`}>{status}</p>}

      {books.length > 0 && (
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>Type</th>
                <th>Title</th>
                <th>Author</th>
                <th>Size</th>
                <th>Content Preview</th>
                <th>Path</th>
              </tr>
            </thead>
            <tbody>
              {books.map((book, index) => (
                <tr key={index}>
                  <td><span className={`badge ${book.file_type}`}>{book.file_type.toUpperCase()}</span></td>
                  <td>{highlightText(book.title, query)}</td>
                  <td>{highlightText(book.author, query)}</td>
                  <td>{formatSize(book.size_bytes)}</td>
                  <td className="preview-cell">{highlightText(book.content_preview, query)}</td>
                  <td className="path-cell" title={book.path}>{book.path}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export default App;