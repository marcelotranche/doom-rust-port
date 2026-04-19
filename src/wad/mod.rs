//! # Modulo WAD (Where's All the Data)
//!
//! O WAD e o formato de arquivo container do DOOM. Todo o conteudo
//! do jogo — mapas, texturas, sons, sprites — vive dentro de um
//! unico arquivo .wad.
//!
//! ## Estrutura do arquivo
//! ```text
//! +--------------------+
//! |  Header (12 bytes) | <- magic ("IWAD"/"PWAD") + num_lumps + dir_offset
//! +--------------------+
//! |  Dados dos lumps   | <- blocos de dados brutos, sem estrutura fixa
//! +--------------------+
//! |  Diretorio         | <- lista de (offset, tamanho, nome) por lump
//! +--------------------+
//! ```
//!
//! ## Como funciona
//!
//! O DOOM carrega o diretorio do WAD na inicializacao e mantem um
//! array de `LumpInfo` com a posicao e tamanho de cada lump no disco.
//! Quando um subsistema precisa de um lump (ex: textura, mapa), ele
//! chama `read_lump()` que le os bytes do disco sob demanda.
//!
//! Lumps sao identificados por nome (ate 8 caracteres ASCII, case-insensitive).
//! PWADs podem sobrescrever lumps do IWAD — a busca e feita de tras para
//! frente, entao o ultimo lump adicionado com um dado nome tem precedencia.
//!
//! ## Arquivo C original: `w_wad.c` / `w_wad.h`
//!
//! ## Diferencas do C original
//! - Sem zone memory (z_zone.c): Rust gerencia memoria automaticamente
//! - Sem file handles abertos: usamos `BufReader` sob demanda
//! - Nomes de lump sao `[u8; 8]` ao inves de `char[8]`
//! - Busca por nome retorna `Option<usize>` ao inves de -1

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

/// Erros que podem ocorrer ao trabalhar com arquivos WAD.
#[derive(Error, Debug)]
pub enum WadError {
    #[error("Arquivo WAD nao encontrado: {0}")]
    FileNotFound(String),

    #[error("Header WAD invalido: esperado IWAD ou PWAD, encontrado '{0}'")]
    InvalidMagic(String),

    #[error("Lump '{0}' nao encontrado no WAD")]
    LumpNotFound(String),

    #[error("Indice de lump invalido: {index} (total: {total})")]
    InvalidLumpIndex { index: usize, total: usize },

    #[error("Erro de I/O ao ler WAD: {0}")]
    Io(#[from] std::io::Error),
}

/// Tipo do arquivo WAD.
///
/// C original: verificado por `strncmp(header.identification, "IWAD", 4)`
/// em `W_AddFile()`, `w_wad.c` linha ~185
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WadType {
    /// WAD principal do jogo (doom.wad, doom2.wad, freedoom.wad)
    Iwad,
    /// WAD de patch/mod que sobrescreve lumps do IWAD
    Pwad,
}

/// Header do arquivo WAD (12 bytes no disco).
///
/// C original: `wadinfo_t` em `w_wad.h`
/// ```c
/// typedef struct {
///     char identification[4]; // "IWAD" ou "PWAD"
///     int  numlumps;          // quantidade de lumps
///     int  infotableofs;      // offset do diretorio
/// } wadinfo_t;
/// ```
#[derive(Debug, Clone)]
struct WadHeader {
    wad_type: WadType,
    num_lumps: u32,
    dir_offset: u32,
}

/// Informacao sobre um lump no diretorio do WAD.
///
/// C original: `lumpinfo_t` em `w_wad.h`
/// ```c
/// typedef struct {
///     char name[8];
///     int  handle;    // file descriptor (nao usado no port Rust)
///     int  position;  // offset no arquivo
///     int  size;      // tamanho em bytes
/// } lumpinfo_t;
/// ```
#[derive(Debug, Clone)]
pub struct LumpInfo {
    /// Nome do lump (ate 8 bytes ASCII, uppercase, padded com zeros).
    /// C original: `name[8]` em `lumpinfo_t`
    pub name: [u8; 8],

    /// Offset do lump dentro do arquivo WAD (em bytes).
    /// C original: `position` em `lumpinfo_t`
    pub offset: u32,

    /// Tamanho do lump em bytes.
    /// C original: `size` em `lumpinfo_t`
    pub size: u32,

    /// Indice do arquivo WAD de origem (para suportar multiplos WADs).
    /// No C original isso era o `handle` (file descriptor).
    pub wad_index: usize,
}

/// Arquivo WAD carregado em memoria.
///
/// No C original, o estado do WAD era mantido em globals:
/// - `lumpinfo` (array de lumpinfo_t)
/// - `numlumps` (int)
/// - `lumpcache` (array de void*)
///
/// Em Rust, encapsulamos tudo em uma struct com ownership claro.
#[derive(Debug)]
pub struct WadFile {
    /// Tipo do WAD (IWAD ou PWAD)
    pub wad_type: WadType,
    /// Caminho do arquivo no disco
    pub path: PathBuf,
}

/// Sistema de WAD — gerencia um ou mais arquivos WAD.
///
/// Equivalente ao conjunto de globals `lumpinfo`/`numlumps`/`lumpcache`
/// do C original, mas encapsulado em uma struct.
///
/// O DOOM suporta carregar multiplos WADs (um IWAD + varios PWADs).
/// Lumps com o mesmo nome em WADs posteriores sobrescrevem os anteriores
/// (a busca e feita de tras para frente).
///
/// C original: `W_InitMultipleFiles()` em `w_wad.c` linha ~292
#[derive(Debug)]
pub struct WadSystem {
    /// Lista de arquivos WAD carregados
    wad_files: Vec<WadFile>,
    /// Diretorio unificado de todos os lumps
    /// C original: `lumpinfo_t* lumpinfo` (global)
    lumps: Vec<LumpInfo>,
}

impl WadSystem {
    /// Cria um novo sistema WAD vazio.
    pub fn new() -> Self {
        WadSystem {
            wad_files: Vec::new(),
            lumps: Vec::new(),
        }
    }

    /// Adiciona um arquivo WAD ao sistema.
    ///
    /// Le o header e o diretorio do WAD e adiciona todos os lumps
    /// ao diretorio unificado. Pode ser chamada multiplas vezes
    /// para carregar IWAD + PWADs.
    ///
    /// C original: `W_AddFile()` em `w_wad.c` linha ~141
    ///
    /// Diferencas do C:
    /// - Nao suporta o hack de reload (filename com '~')
    /// - Nao suporta single lump files (apenas .wad)
    /// - Retorna Result ao inves de chamar I_Error
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), WadError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|_| {
            WadError::FileNotFound(path.display().to_string())
        })?;
        let mut reader = BufReader::new(file);

        // Ler o header (12 bytes)
        // C original: read(handle, &header, sizeof(header))
        let header = Self::read_header(&mut reader)?;

        let wad_index = self.wad_files.len();
        self.wad_files.push(WadFile {
            wad_type: header.wad_type,
            path: path.to_path_buf(),
        });

        // Ler o diretorio de lumps
        // C original: lseek(handle, header.infotableofs, SEEK_SET)
        //             read(handle, fileinfo, length)
        reader.seek(SeekFrom::Start(header.dir_offset as u64))?;

        for _ in 0..header.num_lumps {
            let lump = Self::read_dir_entry(&mut reader, wad_index)?;
            self.lumps.push(lump);
        }

        log::info!(
            "WAD {:?} carregado: {} lumps de {:?}",
            header.wad_type,
            header.num_lumps,
            path.display()
        );

        Ok(())
    }

    /// Le o header de 12 bytes do WAD.
    ///
    /// Layout no disco (little-endian):
    /// - bytes 0..4: magic ("IWAD" ou "PWAD")
    /// - bytes 4..8: numero de lumps (i32)
    /// - bytes 8..12: offset do diretorio (i32)
    fn read_header<R: Read>(reader: &mut R) -> Result<WadHeader, WadError> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;

        let wad_type = match &magic {
            b"IWAD" => WadType::Iwad,
            b"PWAD" => WadType::Pwad,
            _ => {
                let magic_str = String::from_utf8_lossy(&magic).to_string();
                return Err(WadError::InvalidMagic(magic_str));
            }
        };

        let num_lumps = reader.read_u32::<LittleEndian>()?;
        let dir_offset = reader.read_u32::<LittleEndian>()?;

        Ok(WadHeader {
            wad_type,
            num_lumps,
            dir_offset,
        })
    }

    /// Le uma entrada do diretorio (16 bytes).
    ///
    /// C original: `filelump_t` em `w_wad.h`
    /// ```c
    /// typedef struct {
    ///     int  filepos;  // offset dos dados
    ///     int  size;     // tamanho em bytes
    ///     char name[8];  // nome do lump
    /// } filelump_t;
    /// ```
    fn read_dir_entry<R: Read>(
        reader: &mut R,
        wad_index: usize,
    ) -> Result<LumpInfo, WadError> {
        let offset = reader.read_u32::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;
        let mut name = [0u8; 8];
        reader.read_exact(&mut name)?;

        // Converter para uppercase (o DOOM faz isso em strupr())
        for byte in &mut name {
            if byte.is_ascii_lowercase() {
                *byte = byte.to_ascii_uppercase();
            }
        }

        Ok(LumpInfo {
            name,
            offset,
            size,
            wad_index,
        })
    }

    /// Retorna o numero total de lumps carregados.
    ///
    /// C original: `W_NumLumps()` em `w_wad.c` linha ~339
    pub fn num_lumps(&self) -> usize {
        self.lumps.len()
    }

    /// Busca um lump pelo nome, retornando seu indice.
    ///
    /// A busca e feita de tras para frente (como no DOOM original)
    /// para que lumps de PWADs sobrescrevam os do IWAD.
    ///
    /// Nomes sao case-insensitive e limitados a 8 caracteres.
    ///
    /// C original: `W_CheckNumForName()` em `w_wad.c` linha ~351
    /// Retornava -1 se nao encontrado; em Rust usamos Option.
    pub fn find_lump(&self, name: &str) -> Option<usize> {
        let search = Self::normalize_name(name);

        // Busca de tras para frente — PWADs tem precedencia
        // C original: "scan backwards so patch lump files take precedence"
        self.lumps.iter().rposition(|lump| lump.name == search)
    }

    /// Busca um lump pelo nome, retornando erro se nao encontrado.
    ///
    /// C original: `W_GetNumForName()` em `w_wad.c` linha ~399
    /// Chamava `I_Error()` se nao encontrado; aqui retornamos Result.
    pub fn get_lump_index(&self, name: &str) -> Result<usize, WadError> {
        self.find_lump(name)
            .ok_or_else(|| WadError::LumpNotFound(name.to_string()))
    }

    /// Retorna informacoes sobre um lump pelo indice.
    pub fn lump_info(&self, index: usize) -> Result<&LumpInfo, WadError> {
        self.lumps.get(index).ok_or(WadError::InvalidLumpIndex {
            index,
            total: self.lumps.len(),
        })
    }

    /// Retorna o tamanho de um lump em bytes.
    ///
    /// C original: `W_LumpLength()` em `w_wad.c` linha ~416
    pub fn lump_length(&self, index: usize) -> Result<usize, WadError> {
        Ok(self.lump_info(index)?.size as usize)
    }

    /// Le os dados de um lump do disco e retorna como Vec<u8>.
    ///
    /// C original: `W_ReadLump()` em `w_wad.c` linha ~431
    ///
    /// Diferencas do C:
    /// - Retorna Vec<u8> (owned) ao inves de escrever em buffer pre-alocado
    /// - Abre o arquivo sob demanda ao inves de manter file handles abertos
    /// - Usa Result ao inves de I_Error
    pub fn read_lump(&self, index: usize) -> Result<Vec<u8>, WadError> {
        let info = self.lump_info(index)?;
        let wad = &self.wad_files[info.wad_index];

        let file = File::open(&wad.path)?;
        let mut reader = BufReader::new(file);

        reader.seek(SeekFrom::Start(info.offset as u64))?;

        let mut data = vec![0u8; info.size as usize];
        reader.read_exact(&mut data)?;

        Ok(data)
    }

    /// Le um lump pelo nome.
    ///
    /// Combina `W_GetNumForName()` + `W_ReadLump()`.
    /// C original: `W_CacheLumpName()` em `w_wad.c` linha ~508
    pub fn read_lump_by_name(&self, name: &str) -> Result<Vec<u8>, WadError> {
        let index = self.get_lump_index(name)?;
        self.read_lump(index)
    }

    /// Retorna o nome de um lump como string (sem zeros de padding).
    pub fn lump_name(&self, index: usize) -> Result<String, WadError> {
        let info = self.lump_info(index)?;
        let end = info.name.iter().position(|&b| b == 0).unwrap_or(8);
        Ok(String::from_utf8_lossy(&info.name[..end]).to_string())
    }

    /// Normaliza um nome de lump: uppercase, padded com zeros ate 8 bytes.
    ///
    /// C original: `strupr()` + `strncpy()` em `W_CheckNumForName()`
    fn normalize_name(name: &str) -> [u8; 8] {
        let mut result = [0u8; 8];
        for (i, byte) in name.bytes().take(8).enumerate() {
            result[i] = byte.to_ascii_uppercase();
        }
        result
    }

    /// Retorna uma lista com os nomes de todos os lumps.
    /// Util para debug e exploracao do conteudo do WAD.
    pub fn list_lumps(&self) -> Vec<String> {
        (0..self.lumps.len())
            .map(|i| {
                let name = self.lump_name(i).unwrap_or_default();
                let size = self.lumps[i].size;
                format!("{:>5}: {:8} ({} bytes)", i, name, size)
            })
            .collect()
    }

    /// Retorna o tipo do primeiro WAD carregado (IWAD ou PWAD).
    pub fn wad_type(&self) -> Option<WadType> {
        self.wad_files.first().map(|w| w.wad_type)
    }
}

impl Default for WadSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que normalize_name converte para uppercase e preenche com zeros.
    #[test]
    fn normalize_name_basic() {
        assert_eq!(
            WadSystem::normalize_name("e1m1"),
            [b'E', b'1', b'M', b'1', 0, 0, 0, 0]
        );
    }

    /// Verifica que nomes longos sao truncados em 8 caracteres.
    #[test]
    fn normalize_name_truncate() {
        let result = WadSystem::normalize_name("LONGERNAME");
        assert_eq!(result, [b'L', b'O', b'N', b'G', b'E', b'R', b'N', b'A']);
    }

    /// Verifica que o sistema WAD inicia vazio.
    #[test]
    fn empty_wad_system() {
        let wad = WadSystem::new();
        assert_eq!(wad.num_lumps(), 0);
        assert!(wad.find_lump("E1M1").is_none());
    }

    /// Verifica que buscar lump em WAD vazio retorna erro.
    #[test]
    fn get_lump_not_found() {
        let wad = WadSystem::new();
        assert!(matches!(
            wad.get_lump_index("E1M1"),
            Err(WadError::LumpNotFound(_))
        ));
    }

    /// Verifica que abrir arquivo inexistente retorna erro.
    #[test]
    fn add_nonexistent_file() {
        let mut wad = WadSystem::new();
        assert!(matches!(
            wad.add_file("/tmp/nao_existe.wad"),
            Err(WadError::FileNotFound(_))
        ));
    }

    /// Verifica que o header de um WAD sintetico e lido corretamente.
    #[test]
    fn read_header_iwad() {
        // Header minimo: "IWAD" + 0 lumps + offset 12
        let data: Vec<u8> = vec![
            b'I', b'W', b'A', b'D', // magic
            0, 0, 0, 0, // num_lumps = 0
            12, 0, 0, 0, // dir_offset = 12
        ];
        let mut cursor = std::io::Cursor::new(data);
        let header = WadSystem::read_header(&mut cursor).unwrap();
        assert_eq!(header.wad_type, WadType::Iwad);
        assert_eq!(header.num_lumps, 0);
        assert_eq!(header.dir_offset, 12);
    }

    /// Verifica que header com magic invalido retorna erro.
    #[test]
    fn read_header_invalid_magic() {
        let data: Vec<u8> = vec![
            b'N', b'O', b'P', b'E', // magic invalido
            0, 0, 0, 0, 12, 0, 0, 0,
        ];
        let mut cursor = std::io::Cursor::new(data);
        assert!(matches!(
            WadSystem::read_header(&mut cursor),
            Err(WadError::InvalidMagic(_))
        ));
    }

    /// Verifica leitura de uma entrada do diretorio.
    #[test]
    fn read_dir_entry_basic() {
        // Uma entrada: offset=100, size=200, name="PLAYPAL\0"
        let data: Vec<u8> = vec![
            100, 0, 0, 0, // offset = 100
            200, 0, 0, 0, // size = 200
            b'P', b'L', b'A', b'Y', b'P', b'A', b'L', 0, // name
        ];
        let mut cursor = std::io::Cursor::new(data);
        let entry = WadSystem::read_dir_entry(&mut cursor, 0).unwrap();
        assert_eq!(entry.offset, 100);
        assert_eq!(entry.size, 200);
        assert_eq!(&entry.name, b"PLAYPAL\0");
    }
}
