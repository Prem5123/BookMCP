# Release license fallbacks

The release packager copies LICENSE, NOTICE, and equivalent files from every resolved dependency package. Some crates.io packages omit these files. `sources.json` records reviewed repository fallbacks and upstream commit IDs; bundled upstream license and notice files are unchanged.

`adobe-cmap-parser`, `pdf-extract`, `type1-encoding-parser`, and `htmlescape` declare MIT licensing in their published Cargo manifests but contain no standalone license text in the package or the reviewed upstream repository root. For these packages, the archive includes the original Cargo manifest, its declared authors/license, and the standard MIT terms from `MIT.txt`. This supplied standard text is not presented as a verbatim upstream file. The legacy slash-separated `htmlescape` license expression offers MIT as one of its alternatives.

When updating dependencies, review these fallbacks and source references. Packaging fails for a newly missing dependency license instead of silently producing an archive without notices.
