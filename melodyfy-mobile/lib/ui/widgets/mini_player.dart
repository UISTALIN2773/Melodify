import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../providers/player_provider.dart';
import '../screens/player_screen.dart';
import 'spectrum_visualizer.dart';

class MiniPlayer extends StatelessWidget {
  const MiniPlayer({super.key});

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<PlayerProvider>();
    final track = provider.currentTrack;
    if (track == null) return const SizedBox.shrink();

    final accent = provider.accentColor;
    final durMs = provider.duration.inMilliseconds > 0
        ? provider.duration.inMilliseconds
        : (track.durationSeconds * 1000);
    final progress = durMs > 0
        ? (provider.position.inMilliseconds / durMs).clamp(0.0, 1.0)
        : 0.0;
    final isFav = provider.library.isFavorite(track.id);

    return GestureDetector(
      onTap: () {
        Navigator.of(context).push(
          MaterialPageRoute(builder: (_) => const PlayerScreen()),
        );
      },
      onHorizontalDragEnd: (details) {
        if ((details.primaryVelocity ?? 0) < -300) {
          provider.playNext();
        } else if ((details.primaryVelocity ?? 0) > 300) {
          provider.playPrev();
        }
      },
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: const Color(0xFF232323),
          borderRadius: BorderRadius.circular(10),
          boxShadow: [
            BoxShadow(
              color: Colors.black.withValues(alpha: 0.45),
              blurRadius: 10,
              offset: const Offset(0, 4),
            ),
          ],
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 7, 6, 5),
              child: Row(
                children: [
                  ClipRRect(
                    borderRadius: BorderRadius.circular(6),
                    child: SizedBox(
                      width: 42,
                      height: 42,
                      child: track.thumbnailUrl.isNotEmpty
                          ? CachedNetworkImage(
                              imageUrl: track.thumbnailUrl,
                              fit: BoxFit.cover,
                              errorWidget: (_, _, _) => Image.asset(
                                  'assets/logo.jpg',
                                  fit: BoxFit.cover),
                            )
                          : Image.asset('assets/logo.jpg', fit: BoxFit.cover),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Expanded(
                              child: Text(
                                track.title,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(
                                  color: Colors.white,
                                  fontWeight: FontWeight.bold,
                                  fontSize: 13,
                                ),
                              ),
                            ),
                            if (provider.isPlaying) ...[
                              const SizedBox(width: 6),
                              SpectrumVisualizer(
                                isPlaying: provider.isPlaying,
                                color: accent,
                                barCount: 4,
                                height: 12,
                                barWidth: 2.5,
                              ),
                            ],
                          ],
                        ),
                        Text(
                          provider.isLoadingStream
                              ? '⚡ Cargando audio...'
                              : track.artist,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: provider.isLoadingStream
                                ? accent
                                : Colors.grey.shade400,
                            fontSize: 11,
                          ),
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    icon: Icon(
                      isFav ? Icons.favorite : Icons.favorite_border,
                      color: isFav ? accent : Colors.white70,
                      size: 20,
                    ),
                    onPressed: () => provider.toggleFavorite(track),
                  ),
                  IconButton(
                    icon: Icon(
                      provider.isPlaying
                          ? Icons.pause_rounded
                          : Icons.play_arrow_rounded,
                      color: Colors.white,
                      size: 28,
                    ),
                    onPressed: () => provider.togglePlayPause(),
                  ),
                ],
              ),
            ),
            ClipRRect(
              borderRadius:
                  const BorderRadius.vertical(bottom: Radius.circular(10)),
              child: LinearProgressIndicator(
                value: progress,
                minHeight: 2.5,
                backgroundColor: Colors.white12,
                valueColor: AlwaysStoppedAnimation<Color>(accent),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
