import 'dart:convert';
import 'dart:io';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../models/track.dart';

class LibraryService {
  static const _storageKey = 'melodyfy_mobile_library_v1';

  List<Track> favorites = [];
  List<Track> history = [];
  Map<String, List<Track>> playlists = {};
  List<Track> downloads = [];
  List<QuickTag> quickTags = QuickTag.defaults();
  bool autoRadio = true;
  bool useSpotifyGreen = true;

  Future<void> load() async {
    final prefs = await SharedPreferences.getInstance();
    final raw = prefs.getString(_storageKey);
    if (raw != null && raw.isNotEmpty) {
      try {
        final map = jsonDecode(raw) as Map<String, dynamic>;
        favorites = (map['favorites'] as List<dynamic>? ?? [])
            .map((e) => Track.fromJson(e as Map<String, dynamic>))
            .toList();
        history = (map['history'] as List<dynamic>? ?? [])
            .map((e) => Track.fromJson(e as Map<String, dynamic>))
            .toList();
        downloads = (map['downloads'] as List<dynamic>? ?? [])
            .map((e) => Track.fromJson(e as Map<String, dynamic>))
            .toList();
        final rawPl = map['playlists'] as Map<String, dynamic>? ?? {};
        playlists = rawPl.map((k, v) => MapEntry(
              k,
              (v as List<dynamic>)
                  .map((e) => Track.fromJson(e as Map<String, dynamic>))
                  .toList(),
            ));
        if (map['quick_tags'] != null) {
          quickTags = (map['quick_tags'] as List<dynamic>)
              .map((e) => QuickTag.fromJson(e as Map<String, dynamic>))
              .toList();
        }
        autoRadio = (map['auto_radio'] as bool?) ?? true;
        useSpotifyGreen = (map['use_spotify_green'] as bool?) ?? true;
      } catch (_) {}
    }
    await scanLocalDownloads();
  }

  Future<void> save() async {
    final prefs = await SharedPreferences.getInstance();
    final map = {
      'favorites': favorites.map((t) => t.toJson()).toList(),
      'history': history.map((t) => t.toJson()).toList(),
      'downloads': downloads.map((t) => t.toJson()).toList(),
      'playlists': playlists.map(
        (k, v) => MapEntry(k, v.map((t) => t.toJson()).toList()),
      ),
      'quick_tags': quickTags.map((t) => t.toJson()).toList(),
      'auto_radio': autoRadio,
      'use_spotify_green': useSpotifyGreen,
    };
    await prefs.setString(_storageKey, jsonEncode(map));
  }

  Future<Directory> getDownloadsDir() async {
    final appDir = await getApplicationDocumentsDirectory();
    final dir = Directory('${appDir.path}/MelodyfyDownloads');
    if (!await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  Future<void> scanLocalDownloads() async {
    final dir = await getDownloadsDir();
    final files = dir.listSync().whereType<File>().toList();
    bool changed = false;

    for (final file in files) {
      final path = file.path;
      final lower = path.toLowerCase();
      if (!lower.endsWith('.m4a') &&
          !lower.endsWith('.mp3') &&
          !lower.endsWith('.aac') &&
          !lower.endsWith('.wav')) {
        continue;
      }
      final localUrl = 'local:$path';
      if (!downloads.any((t) => t.webpageUrl == localUrl)) {
        final name = path.split(Platform.pathSeparator).last;
        final stem = name.contains('.') ? name.substring(0, name.lastIndexOf('.')) : name;
        final parts = stem.split(' - ');
        final artist = parts.length > 1 ? parts[0].trim() : 'Audio Offline';
        final title = parts.length > 1 ? parts.sublist(1).join(' - ').trim() : stem;
        downloads.add(Track(
          id: 'local_$stem',
          title: title,
          artist: artist,
          duration: 'Offline',
          durationSeconds: 0,
          thumbnailUrl: '',
          webpageUrl: localUrl,
        ));
        changed = true;
      }
    }

    final beforeLen = downloads.length;
    downloads.removeWhere((t) => t.isLocal && !File(t.localPath).existsSync());
    if (changed || downloads.length != beforeLen) {
      await save();
    }
  }

  bool isFavorite(String id) => favorites.any((t) => t.id == id);

  bool toggleFavorite(Track track) {
    final idx = favorites.indexWhere((t) => t.id == track.id);
    if (idx >= 0) {
      favorites.removeAt(idx);
      save();
      return false;
    } else {
      favorites.insert(0, track);
      save();
      return true;
    }
  }

  void recordHistory(Track track) {
    history.removeWhere((t) => t.id == track.id);
    history.insert(0, track);
    if (history.length > 100) {
      history = history.sublist(0, 100);
    }
    save();
  }

  bool createPlaylist(String name) {
    final clean = name.trim();
    if (clean.isEmpty || playlists.containsKey(clean)) return false;
    playlists[clean] = [];
    save();
    return true;
  }

  void deletePlaylist(String name) {
    if (playlists.remove(name) != null) save();
  }

  bool addToPlaylist(String playlistName, Track track) {
    final list = playlists.putIfAbsent(playlistName, () => []);
    if (!list.any((t) => t.id == track.id)) {
      list.add(track);
      save();
      return true;
    }
    return false;
  }

  void removeFromPlaylist(String playlistName, String trackId) {
    final list = playlists[playlistName];
    if (list != null) {
      list.removeWhere((t) => t.id == trackId);
      save();
    }
  }

  void addQuickTag(String label, String query) {
    quickTags.removeWhere((t) => t.label == label);
    quickTags.add(QuickTag(label: label, query: query));
    save();
  }

  void removeQuickTag(int index) {
    if (index >= 0 && index < quickTags.length) {
      quickTags.removeAt(index);
      save();
    }
  }

  void recordDownload(Track track, String localPath) {
    final offlineTrack = track.copyWith(webpageUrl: 'local:$localPath');
    downloads.removeWhere(
        (t) => t.id == track.id || t.webpageUrl == offlineTrack.webpageUrl);
    downloads.insert(0, offlineTrack);
    save();
  }
}
