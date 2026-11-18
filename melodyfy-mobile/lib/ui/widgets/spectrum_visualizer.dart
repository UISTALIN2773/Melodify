import 'dart:math';
import 'package:flutter/material.dart';

class SpectrumVisualizer extends StatefulWidget {
  final bool isPlaying;
  final Color color;
  final int barCount;
  final double height;
  final double barWidth;

  const SpectrumVisualizer({
    super.key,
    required this.isPlaying,
    required this.color,
    this.barCount = 4,
    this.height = 16,
    this.barWidth = 3,
  });

  @override
  State<SpectrumVisualizer> createState() => _SpectrumVisualizerState();
}

class _SpectrumVisualizerState extends State<SpectrumVisualizer>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1200),
    );
    if (widget.isPlaying) _controller.repeat();
  }

  @override
  void didUpdateWidget(covariant SpectrumVisualizer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.isPlaying && !_controller.isAnimating) {
      _controller.repeat();
    } else if (!widget.isPlaying && _controller.isAnimating) {
      _controller.stop();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _controller,
      builder: (context, _) {
        final t = _controller.value * 2 * pi;
        return SizedBox(
          height: widget.height,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.end,
            children: List.generate(widget.barCount, (i) {
              final factor = widget.isPlaying
                  ? (0.25 + 0.75 * sin(t * (2 + (i % 5)) + i * 0.7).abs())
                  : 0.2;
              return Container(
                width: widget.barWidth,
                height: (widget.height * factor).clamp(3.0, widget.height),
                margin: const EdgeInsets.symmetric(horizontal: 1.5),
                decoration: BoxDecoration(
                  color: widget.color,
                  borderRadius: BorderRadius.circular(2),
                ),
              );
            }),
          ),
        );
      },
    );
  }
}
