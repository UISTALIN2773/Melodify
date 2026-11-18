import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../providers/player_provider.dart';
import '../widgets/spectrum_visualizer.dart';

class LyricsScreen extends StatefulWidget {
  const LyricsScreen({super.key});

  @override
  State<LyricsScreen> createState() => _LyricsScreenState();
}

class _LyricsScreenState extends State<LyricsScreen> {
  final ScrollController _scrollController = ScrollController();
  int _lastActiveIdx = -1;

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<PlayerProvider>();
    final track = provider.currentTrack;
    final accent = provider.accentColor;

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 12, 8),
          child: Row(
            children: [
              if (Navigator.canPop(context))
                IconButton(
                  icon: const Icon(Icons.arrow_back, color: Colors.white),
                  onPressed: () => Navigator.pop(context),
                ),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      track?.title ?? '🎤 Letras Karaoke LRCLIB',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
                        fontSize: 18,
                      ),
                    ),
                    if (track != null)
                      Text(
                        track.artist,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                            color: Colors.grey.shade400, fontSize: 12),
                      ),
                  ],
                ),
              ),
              IconButton(
                icon: const Icon(Icons.refresh, color: Colors.white70),
                onPressed: () => provider.fetchLyricsForCurrent(force: true),
              ),
            ],
          ),
        ),
        if (track != null)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                if (provider.isPlaying)
                  SpectrumVisualizer(
                    isPlaying: true,
                    color: accent,
                    barCount: 20,
                    height: 18,
                    barWidth: 3.5,
                  )
                else
                  const SizedBox(),
                Row(
                  children: [
                    TextButton(
                      onPressed: () => provider.adjustLyricsOffset(-0.5),
                      child: const Text('⏪ -0.5s',
                          style: TextStyle(color: Colors.white70, fontSize: 12)),
                    ),
                    GestureDetector(
                      onTap: provider.resetLyricsOffset,
                      child: Text(
                        '${provider.lyricsOffsetSecs >= 0 ? '+' : ''}${provider.lyricsOffsetSecs.toStringAsFixed(1)}s',
                        style: TextStyle(color: accent, fontSize: 12),
                      ),
                    ),
                    TextButton(
                      onPressed: () => provider.adjustLyricsOffset(0.5),
                      child: const Text('⏩ +0.5s',
                          style: TextStyle(color: Colors.white70, fontSize: 12)),
                    ),
                  ],
                ),
              ],
            ),
          ),
        const Divider(color: Color(0xFF282828)),
        Expanded(
          child: _buildBody(provider, accent),
        ),
      ],
    );
  }

  Widget _buildBody(PlayerProvider provider, Color accent) {
    if (provider.currentTrack == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(18),
              child: Image.asset('assets/logo.jpg',
                  width: 110, height: 110, fit: BoxFit.cover),
            ),
            const SizedBox(height: 16),
            const Text(
              'Elige una canción para ver su letra sincronizada',
              style: TextStyle(color: Colors.grey, fontSize: 15),
            ),
          ],
        ),
      );
    }

    if (provider.isLoadingLyrics) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            CircularProgressIndicator(color: accent),
            const SizedBox(height: 12),
            Text('Buscando letra en LRCLIB...',
                style: TextStyle(color: accent)),
          ],
        ),
      );
    }

    final lyrics = provider.lyricsData;
    if (lyrics == null || lyrics.lines.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('No se encontró letra sincronizada para esta pista.',
                style: TextStyle(color: Colors.grey)),
            const SizedBox(height: 12),
            ElevatedButton(
              style: ElevatedButton.styleFrom(
                  backgroundColor: accent, foregroundColor: Colors.black),
              onPressed: () => provider.fetchLyricsForCurrent(force: true),
              child: const Text('Reintentar'),
            ),
          ],
        ),
      );
    }

    final curSecs =
        (provider.position.inMilliseconds / 1000.0) + provider.lyricsOffsetSecs;
    int activeIdx = -1;
    if (lyrics.isSynced) {
      for (int i = 0; i < lyrics.lines.length; i++) {
        if (lyrics.lines[i].timeSeconds <= curSecs) {
          activeIdx = i;
        } else {
          break;
        }
      }
    }

    if (activeIdx != -1 && activeIdx != _lastActiveIdx) {
      _lastActiveIdx = activeIdx;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_scrollController.hasClients) {
          final target = (activeIdx * 54.0 - 140.0)
              .clamp(0.0, _scrollController.position.maxScrollExtent);
          _scrollController.animateTo(
            target,
            duration: const Duration(milliseconds: 300),
            curve: Curves.easeOut,
          );
        }
      });
    }

    return ListView.builder(
      controller: _scrollController,
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 24),
      itemCount: lyrics.lines.length,
      itemBuilder: (_, i) {
        final line = lyrics.lines[i];
        final isActive = i == activeIdx;
        return InkWell(
          onTap: lyrics.isSynced
              ? () => provider.seek(
                  Duration(milliseconds: (line.timeSeconds * 1000).round()))
              : null,
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 10),
            child: Text(
              line.text.isEmpty ? '♪' : line.text,
              textAlign: TextAlign.center,
              style: TextStyle(
                color: isActive ? Colors.white : Colors.grey.shade600,
                fontSize: isActive ? 22 : 17,
                fontWeight: isActive ? FontWeight.bold : FontWeight.w500,
              ),
            ),
          ),
        );
      },
    );
  }
}
