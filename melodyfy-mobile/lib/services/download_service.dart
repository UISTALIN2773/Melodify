import 'dart:io';
import 'package:http/http.dart' as http;
import '../models/track.dart';
import 'innertube_service.dart';
import 'library_service.dart';

class DownloadService {
  final InnerTubeService innerTube;
  final LibraryService library;

  DownloadService({required this.innerTube, required this.library});

  Future<String?> downloadTrack(Track track) async {
    try {
      final streamUrl = await innerTube.resolveStreamUrl(track);
      if (streamUrl == null || streamUrl.isEmpty) return null;

      final dir = await library.getDownloadsDir();
      final cleanArtist = _sanitize(track.artist);
      final cleanTitle = _sanitize(track.title);
      final filePath = '${dir.path}/$cleanArtist - $cleanTitle.m4a';

      final req = http.Request('GET', Uri.parse(streamUrl));
      final streamedResp = await http.Client().send(req);
      if (streamedResp.statusCode < 200 || streamedResp.statusCode >= 300) {
        return null;
      }

      final file = File(filePath);
      final sink = file.openWrite();
      await streamedResp.stream.pipe(sink);
      await sink.close();

      library.recordDownload(track, filePath);
      return filePath;
    } catch (_) {
      return null;
    }
  }

  String _sanitize(String input) {
    return input.replaceAll(RegExp(r'[\\/:*?"<>|]'), '_').trim();
  }
}
