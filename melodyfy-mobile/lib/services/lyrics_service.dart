import 'dart:convert';
import 'package:http/http.dart' as http;
import '../models/track.dart';

class LyricsService {
  static const _userAgent = 'Melodyfy-Mobile-Android/2.0 (https://github.com/UISTALIN2773/Melodify)';

  Future<LyricsData?> fetchLyrics(Track track) async {
    final cleanTitle = _cleanTitle(track.title);
    final cleanArtist = _cleanArtist(track.artist);

    final exact = await _queryGet(cleanTitle, cleanArtist, track.durationSeconds, track.id);
    if (exact != null) return exact;

    return _querySearch(cleanTitle, cleanArtist, track.id);
  }

  String _cleanTitle(String title) {
    var t = title;
    const patterns = [
      '(Official Video)',
      '(Official Audio)',
      '(Official Music Video)',
      '[Official Video]',
      '[Official Audio]',
      '[Official Music Video]',
      '(Audio)',
      '[Audio]',
      '(Video)',
      '[Video]',
      '(Lyric Video)',
      '[Lyric Video]',
      '(Lyrics)',
      '[Lyrics]',
      '(HD)',
      '[HD]',
      '(4K)',
      '[4K]',
      '(MV)',
      '[MV]',
      'MV',
      'Video Oficial',
      'Audio Oficial',
      'Letra',
    ];
    for (final p in patterns) {
      t = t.replaceAll(p, '');
    }
    final splitIdx = t.indexOf(' - ');
    if (splitIdx != -1) {
      t = t.substring(splitIdx + 3);
    }
    return t.trim();
  }

  String _cleanArtist(String artist) {
    var a = artist;
    for (final s in [' - Topic', 'VEVO', 'Official']) {
      a = a.replaceAll(s, '');
    }
    return a.trim();
  }

  Future<LyricsData?> _queryGet(
      String title, String artist, int durationSecs, String trackId) async {
    try {
      final params = <String, String>{
        'track_name': title,
        'artist_name': artist,
      };
      if (durationSecs > 0) {
        params['duration'] = durationSecs.toString();
      }
      final uri = Uri.https('lrclib.net', '/api/get', params);
      final resp = await http
          .get(uri, headers: {'User-Agent': _userAgent})
          .timeout(const Duration(seconds: 6));
      if (resp.statusCode != 200) return null;
      final json = jsonDecode(resp.body) as Map<String, dynamic>;
      return _parseResponse(json, trackId);
    } catch (_) {
      return null;
    }
  }

  Future<LyricsData?> _querySearch(
      String title, String artist, String trackId) async {
    try {
      final uri = Uri.https('lrclib.net', '/api/search', {'q': '$title $artist'});
      final resp = await http
          .get(uri, headers: {'User-Agent': _userAgent})
          .timeout(const Duration(seconds: 6));
      if (resp.statusCode != 200) return null;
      final list = jsonDecode(resp.body) as List<dynamic>;
      for (final item in list) {
        if (item is Map<String, dynamic>) {
          final parsed = _parseResponse(item, trackId);
          if (parsed != null) return parsed;
        }
      }
      return null;
    } catch (_) {
      return null;
    }
  }

  LyricsData? _parseResponse(Map<String, dynamic> json, String trackId) {
    final synced = json['syncedLyrics']?.toString();
    if (synced != null && synced.trim().isNotEmpty) {
      final lines = parseLrc(synced);
      if (lines.isNotEmpty) {
        return LyricsData(
          trackId: trackId,
          isSynced: true,
          lines: lines,
          plainLyrics: json['plainLyrics']?.toString(),
        );
      }
    }

    final plain = json['plainLyrics']?.toString();
    if (plain != null && plain.trim().isNotEmpty) {
      final lines = plain
          .split('\n')
          .map((l) => LyricLine(timeSeconds: 0.0, text: l.trim()))
          .where((l) => l.text.isNotEmpty)
          .toList();
      return LyricsData(
        trackId: trackId,
        isSynced: false,
        lines: lines,
        plainLyrics: plain,
      );
    }
    return null;
  }

  List<LyricLine> parseLrc(String lrcText) {
    final lines = <LyricLine>[];
    for (final raw in lrcText.split('\n')) {
      final line = raw.trim();
      if (!line.startsWith('[')) continue;
      final close = line.indexOf(']');
      if (close == -1) continue;
      final timeStr = line.substring(1, close);
      final text = line.substring(close + 1).trim();
      final secs = _parseTimestamp(timeStr);
      if (secs != null) {
        lines.add(LyricLine(timeSeconds: secs, text: text));
      }
    }
    lines.sort((a, b) => a.timeSeconds.compareTo(b.timeSeconds));
    return lines;
  }

  double? _parseTimestamp(String timeStr) {
    final parts = timeStr.split(':');
    if (parts.length == 2) {
      final m = double.tryParse(parts[0]);
      final s = double.tryParse(parts[1]);
      if (m != null && s != null) return m * 60.0 + s;
    } else if (parts.length == 3) {
      final h = double.tryParse(parts[0]);
      final m = double.tryParse(parts[1]);
      final s = double.tryParse(parts[2]);
      if (h != null && m != null && s != null) {
        return h * 3600.0 + m * 60.0 + s;
      }
    }
    return null;
  }
}
