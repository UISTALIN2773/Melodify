class Track {
  final String id;
  final String title;
  final String artist;
  final String duration;
  final int durationSeconds;
  final String thumbnailUrl;
  final String webpageUrl;

  const Track({
    required this.id,
    required this.title,
    required this.artist,
    required this.duration,
    required this.durationSeconds,
    required this.thumbnailUrl,
    required this.webpageUrl,
  });

  bool get isLocal => webpageUrl.startsWith('local:');
  String get localPath => isLocal ? webpageUrl.substring(6) : '';

  Track copyWith({
    String? id,
    String? title,
    String? artist,
    String? duration,
    int? durationSeconds,
    String? thumbnailUrl,
    String? webpageUrl,
  }) {
    return Track(
      id: id ?? this.id,
      title: title ?? this.title,
      artist: artist ?? this.artist,
      duration: duration ?? this.duration,
      durationSeconds: durationSeconds ?? this.durationSeconds,
      thumbnailUrl: thumbnailUrl ?? this.thumbnailUrl,
      webpageUrl: webpageUrl ?? this.webpageUrl,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'artist': artist,
        'duration': duration,
        'duration_seconds': durationSeconds,
        'thumbnail_url': thumbnailUrl,
        'webpage_url': webpageUrl,
      };

  factory Track.fromJson(Map<String, dynamic> json) {
    return Track(
      id: (json['id'] ?? '').toString(),
      title: (json['title'] ?? 'Unknown Title').toString(),
      artist: (json['artist'] ?? 'Unknown Artist').toString(),
      duration: (json['duration'] ?? '0:00').toString(),
      durationSeconds: (json['duration_seconds'] ?? 0) is int
          ? json['duration_seconds'] as int
          : int.tryParse(json['duration_seconds'].toString()) ?? 0,
      thumbnailUrl: (json['thumbnail_url'] ?? '').toString(),
      webpageUrl: (json['webpage_url'] ?? '').toString(),
    );
  }
}

class QuickTag {
  final String label;
  final String query;

  const QuickTag({required this.label, required this.query});

  Map<String, dynamic> toJson() => {'label': label, 'query': query};

  factory QuickTag.fromJson(Map<String, dynamic> json) => QuickTag(
        label: (json['label'] ?? '').toString(),
        query: (json['query'] ?? '').toString(),
      );

  static List<QuickTag> defaults() => const [
        QuickTag(label: '🔥 Top 50', query: 'Top Éxitos Mundiales 2026'),
        QuickTag(label: '🌸 Anime OPs', query: 'Anime Openings HD'),
        QuickTag(label: '🎧 Lo-Fi Chill', query: 'Lo-Fi Gaming Chill Beats'),
        QuickTag(label: '⚡ Synthwave', query: 'Synthwave Cyberpunk 80s'),
        QuickTag(label: '🎸 Rock Clásico', query: 'Rock Clásico 80s 90s'),
        QuickTag(label: '🎤 J-Pop', query: 'YOASOBI Ado J-Pop Hits'),
        QuickTag(label: '🚗 Phonk Drift', query: 'Phonk Drift'),
        QuickTag(label: '🌃 City Pop', query: 'Japanese City Pop 80s'),
      ];
}

enum RepeatMode { off, all, one }

class LyricLine {
  final double timeSeconds;
  final String text;

  const LyricLine({required this.timeSeconds, required this.text});
}

class LyricsData {
  final String trackId;
  final bool isSynced;
  final List<LyricLine> lines;
  final String? plainLyrics;

  const LyricsData({
    required this.trackId,
    required this.isSynced,
    required this.lines,
    this.plainLyrics,
  });
}
