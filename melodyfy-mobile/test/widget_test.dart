import 'package:flutter_test/flutter_test.dart';
import 'package:melodyfy_mobile/models/track.dart';
import 'package:melodyfy_mobile/services/lyrics_service.dart';

void main() {
  test('Track serialization roundtrip works', () {
    const track = Track(
      id: 'ZRtdQ81jPUQ',
      title: 'Idol',
      artist: 'YOASOBI',
      duration: '3:33',
      durationSeconds: 213,
      thumbnailUrl: 'https://i.ytimg.com/vi/ZRtdQ81jPUQ/hqdefault.jpg',
      webpageUrl: 'https://www.youtube.com/watch?v=ZRtdQ81jPUQ',
    );
    final restored = Track.fromJson(track.toJson());
    expect(restored.id, equals(track.id));
    expect(restored.title, equals(track.title));
    expect(restored.durationSeconds, equals(213));
  });

  test('LRC lyrics parser extracts synced timestamps', () {
    final service = LyricsService();
    final lines = service.parseLrc('[00:12.50] Primera linea\n[01:05.00] Segunda linea');
    expect(lines.length, equals(2));
    expect(lines[0].timeSeconds, closeTo(12.5, 0.01));
    expect(lines[1].timeSeconds, closeTo(65.0, 0.01));
  });
}
