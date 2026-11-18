import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../models/track.dart';
import '../../providers/player_provider.dart';

class TrackTile extends StatelessWidget {
  final Track track;
  final int index;
  final VoidCallback? onTap;
  final String? playlistContextName;

  const TrackTile({
    super.key,
    required this.track,
    required this.index,
    this.onTap,
    this.playlistContextName,
  });

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<PlayerProvider>();
    final accent = provider.accentColor;
    final isCurrent = provider.currentTrack?.id == track.id;
    final isFav = provider.library.isFavorite(track.id);

    return InkWell(
      onTap: onTap ?? () => provider.playTrackNow(track),
      borderRadius: BorderRadius.circular(10),
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 3),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
        decoration: BoxDecoration(
          color: isCurrent ? const Color(0xFF1A2A20) : Colors.transparent,
          borderRadius: BorderRadius.circular(10),
        ),
        child: Row(
          children: [
            SizedBox(
              width: 24,
              child: Text(
                isCurrent
                    ? (provider.isLoadingStream
                        ? '⏳'
                        : (provider.isPlaying ? '⏸' : '▶'))
                    : '${index + 1}'.padLeft(2, '0'),
                style: TextStyle(
                  color: isCurrent ? accent : Colors.grey.shade500,
                  fontWeight: FontWeight.bold,
                  fontSize: 12,
                ),
              ),
            ),
            const SizedBox(width: 8),
            ClipRRect(
              borderRadius: BorderRadius.circular(6),
              child: SizedBox(
                width: 46,
                height: 46,
                child: track.thumbnailUrl.isNotEmpty
                    ? CachedNetworkImage(
                        imageUrl: track.thumbnailUrl,
                        fit: BoxFit.cover,
                        placeholder: (_, _) => Image.asset(
                          'assets/logo.jpg',
                          fit: BoxFit.cover,
                        ),
                        errorWidget: (_, _, _) => Image.asset(
                          'assets/logo.jpg',
                          fit: BoxFit.cover,
                        ),
                      )
                    : Image.asset('assets/logo.jpg', fit: BoxFit.cover),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    track.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      color: isCurrent ? accent : Colors.white,
                      fontWeight: FontWeight.w600,
                      fontSize: 14,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    '${track.artist} • ${track.duration}',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      color: Colors.grey.shade400,
                      fontSize: 11.5,
                    ),
                  ),
                ],
              ),
            ),
            IconButton(
              icon: Icon(
                isFav ? Icons.favorite : Icons.favorite_border,
                color: isFav ? accent : Colors.grey.shade500,
                size: 20,
              ),
              onPressed: () => provider.toggleFavorite(track),
            ),
            IconButton(
              icon: const Icon(Icons.more_vert, color: Colors.grey, size: 20),
              onPressed: () => _showTrackOptions(context, provider),
            ),
          ],
        ),
      ),
    );
  }

  void _showTrackOptions(BuildContext context, PlayerProvider provider) {
    final accent = provider.accentColor;
    final playlists = provider.library.playlists.keys.toList()..sort();

    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF1E1E1E),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox(height: 8),
            Container(
              width: 40,
              height: 4,
              decoration: BoxDecoration(
                color: Colors.grey.shade700,
                borderRadius: BorderRadius.circular(2),
              ),
            ),
            ListTile(
              leading: ClipRRect(
                borderRadius: BorderRadius.circular(6),
                child: track.thumbnailUrl.isNotEmpty
                    ? Image.network(track.thumbnailUrl,
                        width: 44, height: 44, fit: BoxFit.cover,
                        errorBuilder: (_, _, _) =>
                            Image.asset('assets/logo.jpg', width: 44, height: 44))
                    : Image.asset('assets/logo.jpg', width: 44, height: 44),
              ),
              title: Text(track.title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                      color: Colors.white, fontWeight: FontWeight.bold)),
              subtitle: Text(track.artist,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(color: Colors.grey.shade400)),
            ),
            const Divider(color: Color(0xFF2D2D2D)),
            ListTile(
              leading: Icon(Icons.queue_music, color: accent),
              title: const Text('Añadir a la cola',
                  style: TextStyle(color: Colors.white)),
              onTap: () {
                provider.addToQueue(track);
                Navigator.pop(ctx);
              },
            ),
            ListTile(
              leading: Icon(Icons.playlist_add, color: accent),
              title: const Text('Añadir a una Playlist',
                  style: TextStyle(color: Colors.white)),
              onTap: () {
                Navigator.pop(ctx);
                _showPlaylistPicker(context, provider, playlists);
              },
            ),
            if (!track.isLocal)
              ListTile(
                leading: Icon(Icons.download_for_offline, color: accent),
                title: const Text('Descargar Offline (.m4a Sin DRM)',
                    style: TextStyle(color: Colors.white)),
                onTap: () {
                  Navigator.pop(ctx);
                  provider.downloadTrack(track);
                },
              ),
            if (playlistContextName != null)
              ListTile(
                leading: const Icon(Icons.delete_outline, color: Colors.redAccent),
                title: const Text('Quitar de esta Playlist',
                    style: TextStyle(color: Colors.redAccent)),
                onTap: () {
                  provider.library
                      .removeFromPlaylist(playlistContextName!, track.id);
                  provider.showToast('Quitada de $playlistContextName');
                  Navigator.pop(ctx);
                },
              ),
            const SizedBox(height: 12),
          ],
        ),
      ),
    );
  }

  void _showPlaylistPicker(
    BuildContext context,
    PlayerProvider provider,
    List<String> playlists,
  ) {
    final controller = TextEditingController();
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF1E1E1E),
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) => Padding(
        padding: EdgeInsets.only(
          bottom: MediaQuery.of(ctx).viewInsets.bottom + 16,
          left: 16,
          right: 16,
          top: 16,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('Guardar en Playlist',
                style: TextStyle(
                    color: Colors.white,
                    fontSize: 18,
                    fontWeight: FontWeight.bold)),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: controller,
                    style: const TextStyle(color: Colors.white),
                    decoration: const InputDecoration(
                      hintText: 'Nueva playlist...',
                      hintStyle: TextStyle(color: Colors.grey),
                      filled: true,
                      fillColor: Color(0xFF2A2A2A),
                      border: OutlineInputBorder(borderSide: BorderSide.none),
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                ElevatedButton(
                  style: ElevatedButton.styleFrom(
                      backgroundColor: provider.accentColor,
                      foregroundColor: Colors.black),
                  onPressed: () {
                    final name = controller.text.trim();
                    if (name.isNotEmpty) {
                      provider.library.createPlaylist(name);
                      provider.library.addToPlaylist(name, track);
                      provider.showToast('📂 Guardada en $name');
                      Navigator.pop(ctx);
                    }
                  },
                  child: const Text('Crear'),
                ),
              ],
            ),
            const SizedBox(height: 10),
            ...playlists.map(
              (pl) => ListTile(
                leading: Icon(Icons.folder, color: provider.accentColor),
                title: Text(pl, style: const TextStyle(color: Colors.white)),
                onTap: () {
                  provider.library.addToPlaylist(pl, track);
                  provider.showToast('📂 Añadida a $pl');
                  Navigator.pop(ctx);
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}
