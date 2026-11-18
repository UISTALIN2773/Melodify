import 'dart:convert';
import 'package:http/http.dart' as http;
import '../models/track.dart';

class _CachedStream {
  final String url;
  final DateTime createdAt;

  _CachedStream(this.url, this.createdAt);

  bool get isValid => DateTime.now().difference(createdAt).inHours < 5;
}

/// Pure mobile InnerTube HTTP engine (no yt-dlp / Python binary required).
/// Searches YouTube and resolves direct HTTPS audio streams in ~150-300 ms.
class InnerTubeService {
  static const _ua =
      'Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Mobile Safari/537.36';

  final Map<String, _CachedStream> _cache = {};
  final Set<String> _inFlight = {};

  /// Extract YouTube video ID if the user pasted a direct URL
  static String? extractVideoId(String input) {
    final trimmed = input.trim();
    if (!trimmed.startsWith('http://') && !trimmed.startsWith('https://')) {
      return null;
    }
    final uri = Uri.tryParse(trimmed);
    if (uri == null) return null;
    if (uri.host.contains('youtu.be') && uri.pathSegments.isNotEmpty) {
      return uri.pathSegments.first;
    }
    if (uri.queryParameters.containsKey('v')) {
      return uri.queryParameters['v'];
    }
    if (uri.pathSegments.contains('shorts') && uri.pathSegments.length >= 2) {
      final idx = uri.pathSegments.indexOf('shorts');
      if (idx + 1 < uri.pathSegments.length) return uri.pathSegments[idx + 1];
    }
    return null;
  }

  Future<List<Track>> search(String query) async {
    final q = query.trim();
    if (q.isEmpty) return [];

    // Direct YouTube URL support
    final directId = extractVideoId(q);
    if (directId != null && directId.isNotEmpty) {
      final single = await fetchTrackInfo(directId);
      if (single != null) return [single];
    }

    try {
      final payload = {
        'context': {
          'client': {
            'clientName': 'WEB',
            'clientVersion': '2.20240313.01.00',
            'hl': 'es',
            'gl': 'US',
          }
        },
        'query': q,
      };

      final resp = await http
          .post(
            Uri.parse('https://www.youtube.com/youtubei/v1/search'),
            headers: {
              'Content-Type': 'application/json',
              'User-Agent': _ua,
            },
            body: jsonEncode(payload),
          )
          .timeout(const Duration(seconds: 8));

      if (resp.statusCode != 200) return [];
      final data = jsonDecode(resp.body) as Map<String, dynamic>;

      final sections = data['contents']?['twoColumnSearchResultsRenderer']
              ?['primaryContents']?['sectionListRenderer']?['contents']
          as List<dynamic>?;

      if (sections == null) return [];

      final tracks = <Track>[];
      for (final section in sections) {
        final items =
            section['itemSectionRenderer']?['contents'] as List<dynamic>?;
        if (items == null) continue;
        for (final item in items) {
          final vr = item['videoRenderer'] as Map<String, dynamic>?;
          if (vr == null) continue;
          final parsed = _parseVideoRenderer(vr);
          if (parsed != null) tracks.add(parsed);
        }
      }

      if (tracks.isNotEmpty) {
        prefetchStreamUrl(tracks.first);
      }
      return tracks;
    } catch (_) {
      return [];
    }
  }

  Future<Track?> fetchTrackInfo(String videoId) async {
    try {
      final payload = {
        'context': {
          'client': {
            'clientName': 'ANDROID',
            'clientVersion': '19.09.37',
            'androidSdkVersion': 30,
            'hl': 'es',
            'gl': 'US',
          }
        },
        'videoId': videoId,
      };
      final resp = await http
          .post(
            Uri.parse('https://www.youtube.com/youtubei/v1/player'),
            headers: {'Content-Type': 'application/json'},
            body: jsonEncode(payload),
          )
          .timeout(const Duration(seconds: 6));
      if (resp.statusCode != 200) return null;
      final data = jsonDecode(resp.body) as Map<String, dynamic>;
      final details = data['videoDetails'] as Map<String, dynamic>?;
      if (details == null) return null;

      final durSecs = int.tryParse((details['lengthSeconds'] ?? '0').toString()) ?? 0;
      return Track(
        id: videoId,
        title: (details['title'] ?? 'YouTube Audio').toString(),
        artist: (details['author'] ?? 'YouTube').toString(),
        duration: '${durSecs ~/ 60}:${(durSecs % 60).toString().padLeft(2, '0')}',
        durationSeconds: durSecs,
        thumbnailUrl: 'https://i.ytimg.com/vi/$videoId/hqdefault.jpg',
        webpageUrl: 'https://www.youtube.com/watch?v=$videoId',
      );
    } catch (_) {
      return null;
    }
  }

  Track? _parseVideoRenderer(Map<String, dynamic> vr) {
    final id = vr['videoId']?.toString();
    if (id == null || id.isEmpty) return null;

    final titleRuns = vr['title']?['runs'] as List<dynamic>?;
    final title = (titleRuns != null && titleRuns.isNotEmpty)
        ? (titleRuns.first['text'] ?? 'Unknown Title').toString()
        : 'Unknown Title';

    final ownerRuns = (vr['ownerText']?['runs'] ??
        vr['shortBylineText']?['runs']) as List<dynamic>?;
    final artist = (ownerRuns != null && ownerRuns.isNotEmpty)
        ? (ownerRuns.first['text'] ?? 'Unknown Artist').toString()
        : 'Unknown Artist';

    final duration = (vr['lengthText']?['simpleText'] ?? '0:00').toString();
    final durationSeconds = _parseDurationString(duration);

    return Track(
      id: id,
      title: title,
      artist: artist,
      duration: duration,
      durationSeconds: durationSeconds,
      thumbnailUrl: 'https://i.ytimg.com/vi/$id/hqdefault.jpg',
      webpageUrl: 'https://www.youtube.com/watch?v=$id',
    );
  }

  int _parseDurationString(String s) {
    final parts = s.split(':');
    if (parts.length == 2) {
      final m = int.tryParse(parts[0]) ?? 0;
      final sec = int.tryParse(parts[1]) ?? 0;
      return m * 60 + sec;
    } else if (parts.length == 3) {
      final h = int.tryParse(parts[0]) ?? 0;
      final m = int.tryParse(parts[1]) ?? 0;
      final sec = int.tryParse(parts[2]) ?? 0;
      return h * 3600 + m * 60 + sec;
    }
    return 0;
  }

  void prefetchStreamUrl(Track track) {
    if (track.isLocal) return;
    final cached = _cache[track.id];
    if (cached != null && cached.isValid) return;
    if (_inFlight.contains(track.id)) return;
    _inFlight.add(track.id);
    resolveStreamUrl(track).whenComplete(() => _inFlight.remove(track.id));
  }

  Future<String?> resolveStreamUrl(Track track) async {
    if (track.isLocal) return track.localPath;

    final cached = _cache[track.id];
    if (cached != null && cached.isValid) {
      return cached.url;
    }

    // Try ANDROID, then IOS, then ANDROID_MUSIC clients for maximum resilience
    String? url = await _tryClient(
      track.id,
      clientName: 'ANDROID',
      clientVersion: '19.09.37',
      androidSdk: 30,
    );
    url ??= await _tryClient(
      track.id,
      clientName: 'IOS',
      clientVersion: '19.09.3',
    );
    url ??= await _tryClient(
      track.id,
      clientName: 'ANDROID_VR',
      clientVersion: '1.56.21',
      androidSdk: 32,
    );

    if (url != null && url.isNotEmpty) {
      _cache[track.id] = _CachedStream(url, DateTime.now());
    }
    return url;
  }

  Future<String?> _tryClient(
    String videoId, {
    required String clientName,
    required String clientVersion,
    int? androidSdk,
  }) async {
    try {
      final clientObj = <String, dynamic>{
        'clientName': clientName,
        'clientVersion': clientVersion,
        'hl': 'en',
        'gl': 'US',
      };
      if (androidSdk != null) {
        clientObj['androidSdkVersion'] = androidSdk;
      }

      final payload = {
        'context': {'client': clientObj},
        'videoId': videoId,
      };

      final resp = await http
          .post(
            Uri.parse('https://www.youtube.com/youtubei/v1/player'),
            headers: {
              'Content-Type': 'application/json',
              'User-Agent': _ua,
            },
            body: jsonEncode(payload),
          )
          .timeout(const Duration(seconds: 6));

      if (resp.statusCode != 200) return null;
      final data = jsonDecode(resp.body) as Map<String, dynamic>;
      final formats =
          data['streamingData']?['adaptiveFormats'] as List<dynamic>?;
      if (formats == null) return null;

      String? bestM4aUrl;
      int bestM4aBitrate = 0;
      String? bestAnyAudioUrl;
      int bestAnyBitrate = 0;

      for (final f in formats) {
        final fmt = f as Map<String, dynamic>;
        final mime = (fmt['mimeType'] ?? '').toString();
        if (!mime.startsWith('audio/')) continue;
        final url = fmt['url']?.toString();
        if (url == null || url.isEmpty) continue;
        final bitrate = (fmt['bitrate'] ?? 0) is int
            ? fmt['bitrate'] as int
            : int.tryParse(fmt['bitrate'].toString()) ?? 0;

        if (mime.contains('audio/mp4') && bitrate >= bestM4aBitrate) {
          bestM4aBitrate = bitrate;
          bestM4aUrl = url;
        }
        if (bitrate >= bestAnyBitrate) {
          bestAnyBitrate = bitrate;
          bestAnyAudioUrl = url;
        }
      }

      return bestM4aUrl ?? bestAnyAudioUrl;
    } catch (_) {
      return null;
    }
  }
}
