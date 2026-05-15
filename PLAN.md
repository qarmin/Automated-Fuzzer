# Plan: Automated Fuzzer v2

## Obecny stan

Projekt w `old/` - dziala, ale:
- Brak CLI (tylko timeout jako arg)
- Grupowanie bledow zbyt ogolne (np. jeden bucket `assertion` na wszystko)
- Raporty brzydkie, brak automatyzacji zglaszania
- Brak ignore list / walidacji linkow
- CI nie przechowuje danych miedzy runami (brak bazy corpusu/wynikow)
- Brak skilla do tworzenia projektow fuzzerowych

---

## Architektura

```
auto_fuzzer <SUBCOMMAND>

SUBCOMMANDS:
  fuzz          Uruchom fuzzing (custom lub cargo-fuzz)
  minimize      Minimalizuj znalezione pliki (bez usuwania starych)
  report        Generuj raporty z wynikow
  ignore        Dodaj/usun wpis z listy ignorowanych bledow
  validate      Sprawdz statusy linkow w ignore list
  ci            Tryb CI (artefakty, cache, regresje)
```

---

## 1. Dwa tryby fuzzowania

### 1.1. Tryb wlasny (custom)

Jak obecnie - generuje zmutowane pliki (`create_broken_files`), uruchamia program na nich, szuka crashy po output.

```
auto_fuzzer fuzz --mode custom --config fuzz_settings.toml --timeout 3600
```

- Dziala w petli do wyczerpania `--timeout`
- **Graceful stop**: lapie SIGINT/SIGTERM, konczy biezaca iteracje, zapisuje wyniki
- Checkpoint po kazdej iteracji - wyniki sa natychmiast w `broken_files_dir`
- Atomowe operacje na plikach - nigdy nie tracimy danych przy CTRL+C

### 1.2. Tryb cargo-fuzz

Uruchamia `cargo fuzz run` z odpowiednimi parametrami.

```
auto_fuzzer fuzz --mode cargo-fuzz --target image --corpus /opt/INPUT_FILES_DIR --timeout 3600
```

- Wrapper nad `cargo fuzz run ... -- -max_total_time=TIMEOUT`
- Lapie SIGINT - cargo-fuzz sam graceful shutdownuje, potem my przetwarzamy artifacts
- Po zakonczeniu: zbiera pliki z `fuzz/artifacts/<target>/`, filtruje `slow-*`
- **Ujednolicenie wynikow** - oba tryby produkuja ten sam format outputu (patrz sekcja 3)

### 1.3. Wspolne

- `--timeout` - calkowity czas fuzzowania (sekundy), domyslnie bez limitu
- SIGINT/SIGTERM handler z flaga `AtomicBool` - petla sprawdza co iteracje
- Wyniki zapisywane przyrostowo - kill w dowolnym momencie = zachowane dotychczasowe znaleziska

---

## 2. Minimalizacja

```
auto_fuzzer minimize --config fuzz_settings.toml
auto_fuzzer minimize --dir /path/to/broken/files --command "timeout -v 100 lofty {}"
```

- Wywoluje zewnetrzny `minimizer` (juz dziala OK)
- **Nigdy nie usuwa oryginalnych plikow** - minimalizowany plik dostaje suffix `_minimized_N`
- Jesli plik juz ma wersje zminimalizowana - minimalizuje dalej z nowym suffixem
- Struktura katalogu:
  ```
  broken_files/
    crash_abc123.bin           # oryginal
    crash_abc123_minimized.bin # zminimalizowany
  ```

---

## 3. Grupowanie wynikow wg rodzaju bledu

### 3.1. Dokladne grupowanie (Rust-specific)

Obecne grupowanie: `assertion`, `overflow_s`, `panic` - za ogolne.

Nowe podejscie - ekstrakcja **pelnej sygnatury bledu** z outputu:

**Poziom 1: Typ bledu** (jak teraz)
- `assertion`, `overflow_subtract`, `index_out_of_bounds`, `timeout`, itd.

**Poziom 2: Lokalizacja** (nowe - wyciaganie z backtrace/panic message)
- Parsowanie `panicked at 'msg', src/file.rs:123:45` -> `src/file.rs` (bez numeru linii!)
- Parsowanie `assertion failed: WARUNEK` -> pelny warunek
- Parsowanie `left == right failed\n  left: X\n  right: Y` -> typ assertion bez wartosci

**Pelna sygnatura** = `{typ_bledu}::{lokalizacja_bez_linii}::{tresc_bledu_bez_wartosci}`

Przyklady:
```
# "assertion `left == right` failed" w src/wavpack/properties.rs
# -> "assertion_eq::src/wavpack/properties.rs"

# "attempt to subtract with overflow" w src/mpeg/read.rs
# -> "overflow_subtract::src/mpeg/read.rs"

# "index out of bounds: the len is 488 but the index is 488" w src/wavpack/properties.rs
# -> "index_out_of_bounds::src/wavpack/properties.rs"

# "assertion failed: a > 25"
# -> "assertion::a > 25"
# vs "assertion failed: b > 25"
# -> "assertion::b > 25"
# To SA ROZNE BLEDY

# "entered unreachable code: Bad BOM [0, 0]"
# -> "unreachable::Bad BOM"
```

### 3.2. Ekstrakcja lokalizacji z Rust output

Parsowanie panic message:
```rust
// Pattern 1: "panicked at 'MESSAGE', FILE:LINE:COL"
// Pattern 2: "panicked at FILE:LINE:COL:\nMESSAGE"
// Pattern 3: "Source Location: FILE:LINE:COL" (biome style)
// Pattern 4: backtrace z "at FILE:LINE:COL"
```

Reguly:
- Z `src/foo/bar.rs:123:5` bierzemy `src/foo/bar.rs` (bez linii - bo ta sie zmienia miedzy wersjami)
- Z assertion message usuwamy konkretne wartosci liczbowe (`left: 488` -> `left: N`)
- Z "memory allocation of 12345678 bytes" -> "memory_allocation_large"
- Z timeout - grupowanie po samym typie (timeout nie ma lokalizacji)

### 3.3. Struktura wynikow

```
results/
  {projekt}_{sygnatura_hash}/
    to_report.txt              # Raport do zgloszenia
    to_report_metadata.toml    # Metadane (typ, sygnatura, data, wersja)
    problematic_file.{ext}
    compressed.zip
    crash_output.txt           # Pelny output
    reproducer.rs              # (tylko tryb biblioteczny) Kod reprodukujacy
```

Metadane (`to_report_metadata.toml`):
```toml
error_type = "overflow_subtract"
error_signature = "overflow_subtract::src/wavpack/properties.rs"
error_message = "attempt to subtract with overflow"
source_file = "src/wavpack/properties.rs"
project = "lofty"
version = "0.22.0"  # lub commit hash
found_date = "2024-10-30"
file_size = 1234
mode = "custom"  # lub "cargo-fuzz"
```

### 3.4. Deduplikacja

- Przy zapisywaniu nowego crashu - sprawdz czy juz istnieje folder z taka sama sygnatura
- Jesli tak - dodaj plik do istniejacego folderu (jako kolejny reproducer), nie twórz duplikatu
- Osobny plik `signatures.toml` z lista znanych sygnatur i ich folderow

---

## 4. Raporty i zglaszanie bledow

### 4.1. Format raportu

Dwa warianty - CLI (narzedzie uruchamiane z linii polecen) i biblioteka:

#### CLI (np. biome, typst, gdscript-formatter):

Tytul issue: `Panic {krotki_opis_bledu} in {plik_zrodlowy}`

Przyklady tytulow (jak dotychczasowe zgloszenia):
- `Panic attempt to subtract with overflow in src/wavpack/properties.rs`
- `Panic index out of bounds in src/wavpack/properties.rs`
- `Panic internal error: entered unreachable code: Bad BOM in src/id3/v2/read.rs`
- `Timeout when processing specific ogg files`
- `Panic is not a char boundary in src/id3/v2/read.rs`

Reguly tytulow:
- Bez numeru linii
- Z esencja bledu (nie "assertion failed" a "assertion `left == right` failed")
- Jesli overflow/index - dodac plik zrodlowy
- Max ~100 znakow

Tresc:
```markdown
Self compiled {commit_hash}

### What happened?

File content(at the bottom should be attached raw, not formatted file - github removes some non-printable characters, so copying from here may not work):
```
{zawartosc_pliku_jesli_tekst}
```

command
```
{dokladna_komenda_do_reprodukcji}
```

cause this
```
{output_z_crashem}
```

[compressed.zip](attachment)

##### Automatic Fuzzer note, output status "{status}", output signal "{signal}"
```

#### Biblioteka (np. lofty, image, symphonia):

Tytul: `Panic {krotki_opis} in {plik_zrodlowy}` (tak samo)

Tresc:
```markdown
Self compiled {commit_hash}

### Reproducer

I tried this code:

```rust
{kod_programu_reprodukujacego}
```

### Summary

This code causes {opis_problemu} when processing specific {typ_plikow} files.

### Expected behavior

_No response_

### Assets

[compressed.zip](attachment)
```

### 4.2. Automatyczne tworzenie issue

```
auto_fuzzer report create --dir results/lofty_overflow_abc123/
```

Generuje:
1. `issue_title.txt` - gotowy tytul
2. `issue_body.md` - gotowa tresc
3. `compressed.zip` - plik do zalaczenia
4. `create_issue.sh` - skrypt do uruchomienia:
   ```bash
   #!/bin/bash
   # Review the issue before creating!
   # Title: Panic attempt to subtract with overflow in src/wavpack/properties.rs
   gh issue create \
     --repo "Serial-ATA/lofty-rs" \
     --title "$(cat issue_title.txt)" \
     --body "$(cat issue_body.md)"
   echo "UWAGA: Pamietaj o recznym dodaniu compressed.zip jako zalacznika!"
   echo "GitHub CLI nie obsluguje zalaczenikow - dodaj przez interfejs webowy."
   ```

**Wymagane klikniecie uzytkownika** - skrypt nie uruchamia sie sam, trzeba go odpalic recznie po review.

### 4.3. Batch report

```
auto_fuzzer report list                    # Lista wszystkich niezgloszonych crashy
auto_fuzzer report create-all --project lofty  # Generuje issue drafty dla wszystkich
```

---

## 5. Ignore list i walidacja

### 5.1. Ignorowanie znanych bledow

```bash
# Dodaj do ignore list
auto_fuzzer ignore add lofty "Assertion ABCD" "https://github.com/Serial-ATA/lofty-rs/issues/620"
auto_fuzzer ignore add rawler "src/bits.rs" "https://github.com/dnglab/dnglab/issues/571"

# Usun z ignore list
auto_fuzzer ignore remove lofty "Assertion ABCD"

# Lista ignorowanych
auto_fuzzer ignore list
auto_fuzzer ignore list --project lofty
```

Plik `ignore_list.toml`:
```toml
[[ignore]]
project = "lofty"
pattern = "Assertion ABCD"
issue_url = "https://github.com/Serial-ATA/lofty-rs/issues/620"
added_date = "2025-01-15"

[[ignore]]
project = "rawler"
pattern = "src/bits.rs"
issue_url = "https://github.com/dnglab/dnglab/issues/571"
added_date = "2025-02-01"
```

Integracja z fuzzerem:
- Podczas fuzzowania: crashe pasujace do ignore patterns sa pomijane (nie zapisywane)
- Podczas `remove_non_crashing`: pliki pasujace do ignore sa usuwane
- Podczas raportowania: ignorowane crashe nie pojawiaja sie w `report list`
- Pattern matching: substring match na CALYM ouputcie (jak obecne `ignored_item_N`)

### 5.2. Walidacja linkow

```bash
auto_fuzzer validate links
```

Sprawdza kazdy `issue_url` w `ignore_list.toml`:
1. `gh issue view <url> --json state` - pobiera status issue
2. Jesli `state == "closed"` (CLOSED/COMPLETED):
   - Wypisuje: `[FIXED] lofty "Assertion ABCD" - https://github.com/.../issues/620 is CLOSED`
   - Pyta czy usunac z ignore list (lub `--auto-remove` flaga)
3. Jesli `state == "open"`:
   - Wypisuje: `[OPEN] lofty "Assertion ABCD" - still open`
4. Jesli nie mozna pobrac (404, brak dostepu):
   - Wypisuje ostrzezenie

```bash
auto_fuzzer validate links --auto-remove  # Automatycznie usuwa zamkniete
```

---

## 6. Tryb CI (GitHub Actions)

### 6.1. Artefakty i cache miedzy runami

Problem: obecnie kazdy CI run zaczyna od zera - brak corpusu, brak historii.

Rozwiazanie: `auto_fuzzer ci` subcommand do zarzadzania stanem miedzy runami.

```yaml
# W workflow:
- name: Download previous state
  uses: actions/download-artifact@v4
  with:
    name: STATE___${{ matrix.name }}
    path: /opt/fuzzer_state/
  continue-on-error: true  # Pierwszy run nie ma stanu

- name: Run fuzzer with CI mode
  run: |
    auto_fuzzer ci run \
      --config ${{ matrix.config-file }} \
      --timeout ${{ matrix.timeout }} \
      --state-dir /opt/fuzzer_state/ \
      --output-dir /opt/results/

- name: Upload state for next run
  uses: actions/upload-artifact@v4
  with:
    name: STATE___${{ matrix.name }}
    path: /opt/fuzzer_state/
    overwrite: true
```

### 6.2. Struktura state-dir

```
fuzzer_state/
  corpus/              # Pliki corpusu (cargo-fuzz) lub valid_input + wygenerowane
  known_crashes/       # Znane crashe z poprzednich runow (sygnatury + pliki)
  ignore_list.toml     # Ignorowane bledy
  signatures.toml      # Znane sygnatury bledow
  history.toml         # Historia runow
```

### 6.3. Weryfikacja regresji

Przy kazdym CI run:
1. Zaladuj `known_crashes/` z poprzedniego runu
2. Uruchom fuzzer - znajdz nowe crashe
3. **Sprawdz stare crashe** - uruchom program na starych plikach:
   - Jesli crash dalej wystepuje -> `[STILL BROKEN]`
   - Jesli crash nie wystepuje -> `[FIXED]` - przenies do archiwum
4. Zapisz zaktualizowany stan

```
auto_fuzzer ci verify-regressions --state-dir /opt/fuzzer_state/ --config settings.toml
```

### 6.4. CI output

Artefakty:
- `STATE___<NAME>` - stan do nastepnego runu (corpus + known_crashes + ignore)
- `REPORTS___<NAME>` - raporty z nowych crashy (gotowe do zgloszenia)
- `SUMMARY___<NAME>` - podsumowanie: ile nowych, ile naprawionych, ile znanych

---

## 7. Skill do tworzenia projektow fuzzerowych

Skill dla Claude Code: `/create-fuzzer-project`

### 7.1. Co robi skill

Dla podanej biblioteki Rust:
1. Tworzy projekt Cargo z zaleznoscia na biblioteke (wersja z git, nie crates.io!)
2. Generuje `src/main.rs` - binarke CLI do fuzzowania (tryb custom)
3. Generuje `fuzz/fuzz_targets/` - target cargo-fuzz (tryb cargo-fuzz)
4. Generuje `fuzz_settings.toml` - konfiguracje fuzzera
5. Generuje `ignore_list.toml` - pusta ignore lista

### 7.2. Wymagania

- **Zawsze git dependency** - `library = { git = "https://github.com/..." }`
- **Ujednolicenie** - ten sam kod testujacy w CLI i cargo-fuzz (wspolny modul)
- **Sensowne defaults** - search_item, extensions, timeouty dobrane do typu biblioteki

### 7.3. Przyklady generowanego kodu

CLI binary (`src/main.rs`):
```rust
use std::env;
use std::fs;
use lofty::file::AudioFile;
use lofty::probe::Probe;

fn main() {
    let path = env::args().nth(1).expect("Usage: lofty <file>");
    let data = fs::read(&path).unwrap();
    let cursor = std::io::Cursor::new(&data);
    match Probe::new(cursor).read() {
        Ok(tagged_file) => {
            tagged_file.properties();
            tagged_file.tags();
        }
        Err(_) => {}
    }
}
```

Cargo-fuzz target (`fuzz/fuzz_targets/lofty.rs`):
```rust
#![no_main]
use libfuzzer_sys::{fuzz_target, Corpus};
use lofty::probe::Probe;

fuzz_target!(|data: &[u8]| -> Corpus {
    let cursor = std::io::Cursor::new(data);
    match Probe::new(cursor).read() {
        Ok(tagged_file) => {
            tagged_file.properties();
            tagged_file.tags();
            Corpus::Keep
        }
        Err(_) => Corpus::Reject,
    }
});
```

### 7.4. Workflow skilla

1. User: `/create-fuzzer-project lofty https://github.com/Serial-ATA/lofty-rs`
2. Skill czyta README/docs biblioteki, identyfikuje glowne API
3. Generuje projekt z odpowiednim kodem testujacym
4. User moze iterowac: "dodaj testowanie wiecej typow plikow", "napraw blad kompilacji"
5. Skill pamieta kontekst i dostosowuje oba tryby (custom + cargo-fuzz) jednoczesnie

### 7.5. Dodatkowe mozliwosci skilla

- **Naprawianie bledow kompilacji** - jesli biblioteka sie nie kompiluje, skill proponuje fixy
- **Ulepszanie fuzzera** - po znalezieniu crashy, skill moze zasugerowac rozszerzenie testowanego API
- **Sync wersji** - `cargo update` + sprawdzenie czy cargo-fuzz i custom uzywaja tej samej wersji
- **Generowanie reproducerow** - na podstawie crash output i pliku generuje minimalny `fn main()` lub fuzz_target

---

## 8. Kolejnosc implementacji

### Faza 1: Rdzen (CLI + fuzzing)
1. Nowy projekt Cargo z `clap` (subcommandy)
2. Migracja logiki fuzzowania z `old/` - tryb custom
3. Graceful shutdown (SIGINT/SIGTERM handler)
4. Integracja cargo-fuzz jako drugi tryb
5. Ujednolicony format wynikow

### Faza 2: Grupowanie i raporty
6. Nowy parser sygnatur bledow (Rust-specific)
7. Deduplikacja po sygnaturach
8. Generator raportow (CLI + biblioteka warianty)
9. Tytuly issue wg konwencji (bez linii, z esencja bledu)

### Faza 3: Ignore + walidacja
10. `ignore_list.toml` + CLI do zarzadzania
11. Integracja ignore z fuzzerem i raportami
12. `validate links` z `gh` API

### Faza 4: CI
13. `ci run` - wrapper z state management
14. `ci verify-regressions` - sprawdzanie starych crashy
15. Workflow YAML templates

### Faza 5: Skill
16. Skill `/create-fuzzer-project`
17. Generowanie projektow CLI + cargo-fuzz
18. Iteracyjne ulepszanie

---

## 9. Migracja z old/

- `old/` zostaje jako reference - nie usuwamy
- Nowy kod w glownym katalogu
- Konfiguracja kompatybilna wstecz - istniejace `fuzz_settings.toml` powinny dzialac
- CI workflow stopniowo migrowany na nowy binary

---

## 10. Przykladowy uzycie end-to-end

```bash
# 1. Stworz projekt fuzzerowy
# (w Claude Code) /create-fuzzer-project lofty https://github.com/Serial-ATA/lofty-rs

# 2. Uruchom fuzzing na 2 godziny
auto_fuzzer fuzz --mode custom --config fuzz_settings.toml --timeout 7200
# CTRL+C w dowolnym momencie - wyniki zachowane

# 3. Minimalizuj znalezione pliki
auto_fuzzer minimize --config fuzz_settings.toml

# 4. Zobacz pogrupowane wyniki
auto_fuzzer report list
# OUTPUT:
# [NEW] overflow_subtract::src/wavpack/properties.rs  (3 pliki, min 45 bytes)
# [NEW] index_out_of_bounds::src/wavpack/properties.rs  (1 plik, min 89 bytes)
# [IGNORED] assertion::src/id3/v2/read.rs  (issue #620)

# 5. Wygeneruj raport do zgloszenia
auto_fuzzer report create --dir results/lofty_overflow_subtract_abc123/
# Wygenerowano: issue_title.txt, issue_body.md, compressed.zip, create_issue.sh

# 6. Zglos po review
cat issue_title.txt
# -> "Panic attempt to subtract with overflow in src/wavpack/properties.rs"
bash create_issue.sh
# -> gh issue create ...

# 7. Dodaj do ignore po zgloszeniu
auto_fuzzer ignore add lofty "src/wavpack/properties.rs" "https://github.com/Serial-ATA/lofty-rs/issues/999"

# 8. Pozniej - sprawdz czy naprawione
auto_fuzzer validate links
# [CLOSED] lofty "src/wavpack/properties.rs" - issue #999 is CLOSED -> removing from ignore
```
