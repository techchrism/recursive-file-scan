# Recursive File Scan

A quickly-whipped-together utility to write a directory tree to a sqlite database. This includes:
 - name
 - directory parent
 - modification, access, creation times
 - readonly, unix mode
 - size and blake3 hash for files

This tool was created in rust because I don't trust shell scripts or non-binary file formats with arbitrary file/directory names.

Simplicity is preferred over performance and here are some conscious tradeoffs made:
 - single-threaded directory tree walking, file hashing, and database insertion
 - using sqlite3 over a bespoke format

## Future Work
 - run tests with recursive symlinks
 - create another binary to "export" the db to json or txt
 - performance increase if possible without increasing complexity much
 - add cross-platform compatability (should just be the unix mode as far as I'm aware)