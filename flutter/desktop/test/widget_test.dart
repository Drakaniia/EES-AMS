import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

void main() {
  group('Basic Flutter Tests', () {
    testWidgets('Material AppBar should render correctly', (WidgetTester tester) async {
      // Arrange
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            appBar: AppBar(title: Text('Test App')),
            body: Center(child: Text('Test Body')),
          ),
        ),
      );

      // Assert
      expect(find.text('Test App'), findsOneWidget);
      expect(find.text('Test Body'), findsOneWidget);
    });

    testWidgets('Basic button interaction should work', (WidgetTester tester) async {
      // Arrange
      bool buttonPressed = false;
      
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ElevatedButton(
              onPressed: () => buttonPressed = true,
              child: const Text('Press me'),
            ),
          ),
        ),
      );

      // Act
      await tester.tap(find.byType(ElevatedButton));
      await tester.pump();

      // Assert
      expect(buttonPressed, isTrue);
    });

    testWidgets('Riverpod ProviderScope should work', (WidgetTester tester) async {
      // Arrange
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: Scaffold(
              body: Text('Test with Riverpod'),
            ),
          ),
        ),
      );

      // Assert
      expect(find.text('Test with Riverpod'), findsOneWidget);
    });

    testWidgets('ListTile should render correctly', (WidgetTester tester) async {
      // Arrange
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ListView(
              children: const [
                ListTile(
                  title: Text('Item 1'),
                  subtitle: Text('Subtitle 1'),
                ),
                ListTile(
                  title: Text('Item 2'),
                  subtitle: Text('Subtitle 2'),
                ),
              ],
            ),
          ),
        ),
      );

      // Assert
      expect(find.text('Item 1'), findsOneWidget);
      expect(find.text('Item 2'), findsOneWidget);
      expect(find.text('Subtitle 1'), findsOneWidget);
      expect(find.text('Subtitle 2'), findsOneWidget);
    });

    testWidgets('Card widget should render', (WidgetTester tester) async {
      // Arrange
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Card(
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: const [
                    Text('Card Title', style: TextStyle(fontSize: 18)),
                    SizedBox(height: 8),
                    Text('Card content goes here'),
                  ],
                ),
              ),
            ),
          ),
        ),
      );

      // Assert
      expect(find.text('Card Title'), findsOneWidget);
      expect(find.text('Card content goes here'), findsOneWidget);
      expect(find.byType(Card), findsOneWidget);
    });
  });
}
